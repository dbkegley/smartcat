use std::time::Duration;

use super::request_schemas::{AnthropicPrompt, OpenAiPrompt, PromptFormat};
use super::response_schemas::{AnthropicResponse, OllamaResponse, OpenAiResponse};
use crate::config::api::{ApiClient, ApiConfig, ApiError};
use crate::config::prompt::{Message, Prompt};
use crate::utils::handle_api_response;
use crate::Api;

pub struct ReqwestClient {
    api_config: ApiConfig,
    client: reqwest::blocking::Client,
    prompt: Prompt,
}

impl ReqwestClient {
    pub fn new(api_config: ApiConfig, prompt: Prompt) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(
                api_config
                    .timeout_seconds
                    .map(|t| Duration::from_secs(t.into())),
            )
            .build()
            .expect("Unable to initialize reqwest HTTP client");

        ReqwestClient {
            api_config,
            client,
            prompt,
        }
    }
}

impl ApiClient for ReqwestClient {
    fn do_request(&self) -> Result<Message, ApiError> {
        let prompt_format = match self.prompt.api {
            Api::Ollama
            | Api::Openai
            | Api::AzureOpenai
            | Api::Mistral
            | Api::Groq
            | Api::Cerebras => PromptFormat::OpenAi(OpenAiPrompt::from(self.prompt.clone())),
            Api::Anthropic => PromptFormat::Anthropic(AnthropicPrompt::from(self.prompt.clone())),
            Api::AWSBedrock => PromptFormat::AWSBedrock(AnthropicPrompt::from(self.prompt.clone())),
            Api::AnotherApiForTests => panic!("This api is not made for actual use."),
        };

        let request = self
            .client
            .post(&self.api_config.url)
            .header("Content-Type", "application/json")
            .json(&prompt_format);

        // https://stackoverflow.com/questions/77862683/rust-reqwest-cant-make-a-request
        let request = match self.prompt.api {
            Api::Cerebras => request.header("User-Agent", "CUSTOM_NAME/1.0"),
            _ => request,
        };

        // Add auth if necessary
        let request = match self.prompt.api {
            Api::Openai | Api::Mistral | Api::Groq | Api::Cerebras => request.header(
                "Authorization",
                &format!("Bearer {}", &self.api_config.get_api_key()),
            ),
            Api::AzureOpenai => request.header("api-key", &self.api_config.get_api_key()),
            Api::Anthropic => request
                .header("x-api-key", &self.api_config.get_api_key())
                .header(
                    "anthropic-version",
                    self.api_config.version.as_ref().expect(
                        "version required for Anthropic, please add version key to your api config",
                    ),
                ),
            _ => request,
        };

        let response_text: String = match self.prompt.api {
            Api::Ollama => handle_api_response::<OllamaResponse>(
                request
                    .send()
                    .map_err(|e| ApiError::new(self.prompt.model.clone(), e.to_string()))?,
            ),
            Api::Openai | Api::AzureOpenai | Api::Mistral | Api::Groq | Api::Cerebras => {
                handle_api_response::<OpenAiResponse>(
                    request
                        .send()
                        .map_err(|e| ApiError::new(self.prompt.model.clone(), e.to_string()))?,
                )
            }
            Api::Anthropic => handle_api_response::<AnthropicResponse>(
                request
                    .send()
                    .map_err(|e| ApiError::new(self.prompt.model.clone(), e.to_string()))?,
            ),
            Api::AWSBedrock | Api::AnotherApiForTests => unreachable!(),
        };

        Ok(Message::assistant(&response_text))
    }
}
