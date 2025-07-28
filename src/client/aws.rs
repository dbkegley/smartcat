use log::debug;

use crate::client::request_schemas::AnthropicPrompt;
use crate::config::api::{ApiClient, ApiConfig, ApiError};
use crate::config::prompt::{Message, Prompt};
use crate::Api;

use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::{operation::converse::ConverseOutput, Client as BedrockClient};
use tokio::runtime::Runtime;

pub struct AwsClient {
    api_config: ApiConfig,
    client: BedrockClient,
    prompt: Prompt,
    runtime: Runtime,
}

impl AwsClient {
    pub fn new(api_config: ApiConfig, prompt: Prompt) -> Self {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Err(e) => panic!("AwsClient failed to initialize tokio runtime: {e}"),
            Ok(v) => v,
        };
        let config = runtime
            .block_on(async { aws_config::load_defaults(BehaviorVersion::v2025_01_17()).await });
        let client = BedrockClient::new(&config);

        AwsClient {
            api_config,
            client,
            prompt,
            runtime,
        }
    }

    fn get_converse_output_text(&self, output: ConverseOutput) -> Result<String, ApiError> {
        let text = output
            .output()
            .ok_or(ApiError::new(
                self.prompt.model.clone(),
                "no output".to_string(),
            ))?
            .as_message()
            .map_err(|_| {
                ApiError::new(
                    self.prompt.model.clone(),
                    "output not a message".to_string(),
                )
            })?
            .content()
            .first()
            .ok_or(ApiError::new(
                self.prompt.model.clone(),
                "no content in message".to_string(),
            ))?
            .as_text()
            .map_err(|_| {
                ApiError::new(self.prompt.model.clone(), "content is not text".to_string())
            })?
            .to_string();
        Ok(text)
    }
}

impl ApiClient for AwsClient {
    fn do_request(&self) -> Result<Message, ApiError> {
        let prompt_format = match self.prompt.api {
            Api::AWSBedrock => AnthropicPrompt::from(self.prompt.clone()),
            Api::AnotherApiForTests => panic!("This api is not made for actual use."),
            _ => unreachable!(),
        };

        let result = self.runtime.block_on(async {
            let response = self
                .client
                .converse()
                .model_id(self.prompt.model.as_ref().unwrap())
                .set_messages(Some(prompt_format.into()))
                .send()
                .await;

            match response {
                Ok(output) => {
                    let text = self.get_converse_output_text(output)?;
                    Ok(text)
                }
                Err(e) => {
                    use aws_sdk_bedrockruntime::error::DisplayErrorContext;
                    debug!("error: {}", DisplayErrorContext(&e));

                    Err(e
                        .as_service_error()
                        .map(|e| ApiError::new(self.prompt.model.clone(), e.to_string()))
                        .unwrap_or_else(|| {
                            ApiError::new(
                                self.prompt.model.clone(),
                                "Unknown service error".to_string(),
                            )
                        }))
                }
            }
        });

        match result {
            Ok(response) => Ok(Message::assistant(response.as_str())),
            Err(e) => Err(e),
        }
    }
}
