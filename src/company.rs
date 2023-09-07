use reqwest::Client;
use serde_json::Value;
use tokio::try_join;
// use thiserror::Error;

pub struct Company {
    pub name:           String,
    pub market_cap:     i64,
    pub total_debt:     i64,
    pub free_cash:      i64,
    pub free_cash_flow: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Response error: {0}")]
    ResponseError(#[from] serde_json::Error),

    #[error("Failed to parse market cap")]
    MarketCapParseError,
    #[error("Failed to parse total debt")]
    TotalDebtParseError,
    #[error("Failed to parse free cash")]
    FreeCashParseError,
    #[error("Failed to parse free cash flow")]
    FreeCashFlowParseError,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Company {

    pub async fn from_name_client(name: &str, client: &Client) -> Result<Self> {
        let other_req = format!(
            "https://query2.finance.yahoo.com/v6/finance/quoteSummary/{}?modules=financialData",
            name
        );
        let market_cap_req = format!(
            "https://query2.finance.yahoo.com/v6/finance/quoteSummary/{}?modules=summaryDetail",
            name
        );
    
        let market_cap_resp = client.get(&market_cap_req).send();
        let other_resp = client.get(&other_req).send();
    
        let (market_cap_resp, other_resp) = try_join!(market_cap_resp, other_resp)?;
    
        let market_cap_json: Value = market_cap_resp.json().await?;
        let other_json: Value = other_resp.json().await?;

        let market_cap = market_cap_json["quoteSummary"]["result"][0]["summaryDetail"]["marketCap"]["raw"]
            .as_i64().ok_or(Error::MarketCapParseError)?;

        let total_debt = other_json["quoteSummary"]["result"][0]["financialData"]["totalDebt"]["raw"]
            .as_i64().ok_or(Error::TotalDebtParseError)?;

        let free_cash = other_json["quoteSummary"]["result"][0]["financialData"]["totalCash"]["raw"]
            .as_i64().ok_or(Error::FreeCashParseError)?;

        let free_cash_flow = other_json["quoteSummary"]["result"][0]["financialData"]["freeCashflow"]["raw"]
            .as_i64().ok_or(Error::FreeCashFlowParseError)?;
    

        Ok(Self {
            name: name.to_string(),
            market_cap,
            total_debt,
            free_cash,
            free_cash_flow,
        })
    }
}