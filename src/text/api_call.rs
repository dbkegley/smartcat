use crate::client::{aws::AwsClient, reqwest::ReqwestClient};
use crate::config::{
    api::{Api, ApiClient, ApiConfig, ApiError},
    prompt::{Message, Prompt},
};

use log::debug;

enum Client {
    Aws(AwsClient),
    Reqwest(ReqwestClient),
}

impl ApiClient for Client {
    fn do_request(&self) -> Result<Message, ApiError> {
        match self {
            Client::Aws(client) => client.do_request(),
            Client::Reqwest(client) => client.do_request(),
        }
    }
}

pub fn post_prompt_and_get_answer(
    api_config: ApiConfig,
    prompt: &Prompt,
) -> Result<Message, ApiError> {
    debug!(
        "Trying to reach {:?} with key {:?}",
        api_config.url, api_config.api_key
    );
    debug!("Prompt: {:?}", prompt);

    let mut prompt = prompt.clone();

    if prompt.model.is_none() {
        prompt.model = api_config.default_model.clone()
    }

    // currently not compatible with streams
    prompt.stream = Some(false);

    let client = match prompt.api {
        Api::AWSBedrock => Client::Aws(AwsClient::new(api_config, prompt)),
        _ => Client::Reqwest(ReqwestClient::new(api_config, prompt)),
    };

    client.do_request()
}
