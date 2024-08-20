use crate::json_rpc::zil_methods::ZilMethods;
use config::contracts::STAKEING;
use config::MAIN_URL;
use reqwest;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use zil_errors::ZilliqaErrors;

#[derive(Debug)]
pub struct ZilliqaJsonRPC {
    pub nodes: Vec<String>,
}

impl Default for ZilliqaJsonRPC {
    fn default() -> Self {
        Self::new()
    }
}

impl ZilliqaJsonRPC {
    pub fn new() -> Self {
        let nodes = vec![MAIN_URL.to_string()];
        ZilliqaJsonRPC { nodes }
    }

    pub fn from_vec(nodes: Vec<String>) -> Self {
        ZilliqaJsonRPC { nodes }
    }

    pub async fn bootstrap(node_url: &str) -> Result<Self, ZilliqaErrors> {
        let client = reqwest::Client::new();
        let payload = json!({
            "id": "1",
            "jsonrpc": "2.0",
            "method": ZilMethods::GetSmartContractSubState.to_string(),
            "params": [STAKEING, "ssnlist", []]
        });

        let response: Value = client
            .post(node_url)
            .json(&payload)
            .send()
            .await
            .or(Err(ZilliqaErrors::BadRequest))?
            .json()
            .await
            .or(Err(ZilliqaErrors::FailToParseResponse))?;
        let result = response
            .get("result")
            .ok_or(ZilliqaErrors::FailToParseResponse)?
            .get("ssnlist")
            .ok_or(ZilliqaErrors::FailToParseResponse)?;
        let mut nodes: Vec<String> = result
            .as_object()
            .ok_or(ZilliqaErrors::FailToParseResponse)?
            .keys()
            .filter_map(|addr| {
                result
                    .get(addr)
                    .and_then(|obj| obj.get("arguments"))
                    .and_then(|arr| arr.as_array())
                    .and_then(|arr| arr.get(5))
                    .and_then(|v| v.as_str())
                    .map(|url| url.to_string())
            })
            .collect();

        nodes.push(node_url.to_string());

        Ok(Self { nodes })
    }

    pub async fn reqwest<SR>(&self, payloads: Vec<Value>) -> Result<SR, ZilliqaErrors>
    where
        SR: DeserializeOwned + std::fmt::Debug,
    {
        let client = reqwest::Client::new();

        for url in self.nodes.iter() {
            let res = match client.post::<&str>(url).json(&payloads).send().await {
                Ok(response) => response,
                Err(_) => continue,
            };
            let res = match res.json().await {
                Ok(json) => json,
                Err(e) => {
                    dbg!(e);
                    continue;
                }
            };

            return Ok(res);
        }

        Err(ZilliqaErrors::NetowrkIsDown)
    }

    pub fn build_payload(params: Value, method: ZilMethods) -> Value {
        json!({
            "id": 1,
            "jsonrpc": "2.0",
            "method": method.to_string(),
            "params": params
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ZilliqaJsonRPC;
    use crate::json_rpc::{
        zil_interfaces::{GetBalanceRes, ResultRes},
        zil_methods::ZilMethods,
    };
    use serde_json::json;
    use tokio;

    // #[tokio::test]
    // async fn test_bootstrap() {
    //     let default_url = "https://api.zilliqa.com";
    //     let zil = ZilliqaJsonRPC::bootstrap(default_url).await.unwrap();
    //
    //     assert!(zil.nodes.len() > 1);
    // }

    #[tokio::test]
    async fn test_get_balance() {
        let zil = ZilliqaJsonRPC::new();
        let addr = "7793a8e8c09d189d4d421ce5bc5b3674656c5ac1";
        let payloads = vec![ZilliqaJsonRPC::build_payload(
            json!([addr]),
            ZilMethods::GetBalance,
        )];

        let res: Vec<ResultRes<GetBalanceRes>> = zil.reqwest(payloads).await.unwrap();

        assert!(res.len() == 1);
        assert!(res[0].result.is_some());
        assert!(res[0].error.is_none());
    }
}
