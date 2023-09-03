use serde_json::Value;

async fn ratio_of(ticker: &str, cap_coef: f64) -> Option<f64> {
    let other_req = format!(
        "https://query2.finance.yahoo.com/v6/finance/quoteSummary/{}?modules=financialData",
        ticker
    );
    let market_cap_req = format!(
        "https://query2.finance.yahoo.com/v6/finance/quoteSummary/{}?modules=summaryDetail",
        ticker
    );

    let market_cap_req = reqwest::get(&market_cap_req);
    let other_req = reqwest::get(&other_req);

    let market_cap = match market_cap_req.await {
        Ok(resp) => {
            let json: Value = resp
                .json()
                .await
                .expect(format!("Problem with: {}", ticker).as_str());
            let market_cap = json["quoteSummary"]["result"][0]["summaryDetail"]["marketCap"]["raw"]
                .as_u64()
                .expect(format!("parsing gone wrong of {}", ticker).as_str());
            market_cap
        }

        Err(_) => return None,
    };

    let (debt, cash, cash_flow) = match other_req.await {
        Ok(resp) => {
            let json: Value = resp
                .json()
                .await
                .expect(format!("Problem with: {}", ticker).as_str());

            let debt = json["quoteSummary"]["result"][0]["financialData"]["totalDebt"]["raw"]
                .as_u64()
                .expect(format!("Error parsing debt of {}", ticker).as_str());

            let cash = json["quoteSummary"]["result"][0]["financialData"]["totalCash"]["raw"]
                .as_u64() 
                .expect(format!("Error parsing cash of {}", ticker).as_str());

            let cash_flow = json["quoteSummary"]["result"][0]["financialData"]["freeCashflow"]["raw"]
                .as_u64()
                .expect(format!("Error parsing cash flow of {}", ticker).as_str());

            (debt, cash, cash_flow)
        }
        Err(_) => return None,
    };

    println!("{ticker} has\n\tdebt: {debt}\n\tcash: {cash}\n\tcash_flow: {cash_flow}\n\t{cash_flow}");

    Some((market_cap as f64 * cap_coef + debt as f64 - cash as f64) 
        / cash_flow as f64)
}

#[tokio::main]
async fn main() {
    let ratio = ratio_of("AAPL", 1.4).await; 
    println!("{:?}", ratio);
}
