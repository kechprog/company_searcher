use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;

async fn ratio_of<F>(ticker: String, cap_coef: f64, filter: F) -> Option<(String, f64)> 
where F: FnOnce(u64,u64,u64,u64) -> bool
{
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

            match json["quoteSummary"]["result"][0]["summaryDetail"]["marketCap"]["raw"].as_u64() {
                Some(market_cap) => market_cap,
                None => return None,
            }
        }

        Err(_) => return None,
    };

    let (debt, cash, cash_flow) = match other_req.await {
        Ok(resp) => {
            let json: Value = resp
                .json()
                .await
                .expect(format!("Problem with: {}", ticker).as_str());

            let debt = match json["quoteSummary"]["result"][0]["financialData"]["totalDebt"]["raw"]
                .as_u64()
            {
                Some(debt) => debt,
                None => return None,
            };

            let cash = match json["quoteSummary"]["result"][0]["financialData"]["totalCash"]["raw"]
                .as_u64()
            {
                Some(cash) => cash,
                None => return None,
            };

            let cash_flow = match json["quoteSummary"]["result"][0]["financialData"]["freeCashflow"]
                ["raw"]
                .as_u64()
            {
                Some(cash_flow) => cash_flow,
                None => return None,
            };

            (debt, cash, cash_flow)
        }
        Err(_) => return None,
    };

    // dbugging only
    // println!("{ticker} has\n\tdebt: {debt}\n\tcash: {cash}\n\tcash_flow: {cash_flow}\n\t{cash_flow}");
    if !filter(market_cap, debt, cash, cash_flow) {
        return None;
    }

    Some((
        ticker,
        (market_cap as f64 * cap_coef + debt as f64 - cash as f64) / cash_flow as f64,
    ))
}

fn extract_keys<'a>(contents: &'a String) -> impl Iterator<Item = &'a str> {
    contents
        .split(", '")
        .flat_map(|item| item.split("': '").next())
        .map(|s| s.trim_matches(|c| c == '\'' || c == '{'))
}

#[tokio::main]
async fn main() {
    let contents = fs::read_to_string("./symbols.txt").expect("Cant open/read file"); /* for sake of optimization */
    let all_work = extract_keys(&contents).count();
    let mut work_done = 0_usize;
    let mut symbols = extract_keys(&contents);

    let mut output_file = OpenOptions::new()
        .append(true)
        .write(true)
        .create(true)
        .open("output.txt")
        .expect("Can't open output file");

    let filter = |_market_cap, _debt, _cash, cash_flow|
        (cash_flow > 0);

    loop {
        let mut active_jobs = Vec::with_capacity(100);
        while active_jobs.len() < 80 {
            if let Some(symbol) = symbols.next() {
                active_jobs.push(tokio::spawn(ratio_of(symbol.to_string(), 1.4, filter)));
            } else {
                break;
            }
        }

        if active_jobs.is_empty() {
            break;
        }

        let job = active_jobs.remove(0).await.expect("Problem awaiting");
        work_done += 1;
        println!("progress: {}%", work_done as f64 / all_work as f64 * 100.0);

        if let Some((symbol, ratio)) = job {
            write!(output_file, "{}: {}\n", symbol, ratio).expect("Cant write to output file");
        }
    }
}
