use anyhow::{Result, anyhow};
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
pub struct DepositReq {
    pub user: String,
    pub amount: u64,
}
#[derive(Deserialize, Debug)]
pub struct DepositResp {
    pub ok: bool,
    pub tx: String,
    pub user: String,
    pub vault: String,
    pub protocol: String,
    pub requested: u64,
}

#[derive(Serialize)]
pub struct WithdrawReq {
    pub user: String,
}
#[derive(Deserialize, Debug)]
pub struct WithdrawResp {
    pub ok: bool,
    pub tx: String,
    pub user: String,
}

pub struct KeeperHttp {
    base: String,
    client: Client,
}

impl KeeperHttp {
    pub fn new(base: impl Into<String>) -> Result<Self> {
        Ok(Self {
            base: base.into(),
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?,
        })
    }

    pub fn deposit(&self, user: &str, amount: u64) -> Result<DepositResp> {
        let url = format!("{}/deposit", self.base);
        let req = DepositReq { user: user.to_string(), amount };
        let resp = self.client.post(url)
            .json(&req)
            .send()?
            .error_for_status()?
            .json::<DepositResp>()?;
        Ok(resp)
    }


    pub fn withdraw(&self, user: &str) -> Result<WithdrawResp> {
        let url = format!("{}/withdraw", self.base);
        let req = WithdrawReq { user: user.to_string() };
        let resp = self.client.post(url)
            .json(&req)
            .send()?
            .error_for_status()?
            .json::<WithdrawResp>()?;
        Ok(resp)
    }

    pub fn delete_lender(&self, user: &str) -> Result<Response> {
        let url = format!("{}/lenders/{}", self.base, user);
        let resp =self.client.delete(url).send()?
            .error_for_status()?;
        Ok(resp)
    }
}