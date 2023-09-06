use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::thread;
use reqwest::blocking::Client;

fn ratio_of<F>(client: &Client, ticker: String, cap_coef: f64, filter: F) -> Option<(String, f64)>
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

    let market_cap_resp = client.get(&market_cap_req).send().ok()?;
    let other_resp = client.get(&other_req).send().ok()?;

    let market_cap_json: Value = market_cap_resp.json().ok()?;
    let other_json: Value = other_resp.json().ok()?;

    let market_cap = market_cap_json["quoteSummary"]["result"][0]["summaryDetail"]["marketCap"]["raw"]
        .as_u64()?;
    let debt = other_json["quoteSummary"]["result"][0]["financialData"]["totalDebt"]["raw"].as_u64()?;
    let cash = other_json["quoteSummary"]["result"][0]["financialData"]["totalCash"]["raw"].as_u64()?;
    let cash_flow = other_json["quoteSummary"]["result"][0]["financialData"]["freeCashflow"]["raw"]
        .as_u64()?;

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

fn main() {
    let client = Client::new();
    let contents = fs::read_to_string("./symbols.txt").expect("Cant open/read file");
    let all_work = extract_keys(&contents).count();
    let mut work_done = 0_usize;
    let mut symbols = extract_keys(&contents);

    let mut output_file = OpenOptions::new()
        .append(true)
        .write(true)
        .create(true)
        .open("output.txt")
        .expect("Can't open output file");

    let filter = |_market_cap, _debt, _cash, cash_flow| (cash_flow > 0);

    loop {
        let mut handles = vec![];
        while handles.len() < 8 {
            if let Some(symbol) = symbols.next() {
                let symbol_clone = symbol.to_string();
                let client = client.clone();
                let handle = thread::spawn(move || ratio_of(&client, symbol_clone, 1.4, &filter));
                handles.push(handle);
            } else {
                break;
            }
        }

        if handles.is_empty() {
            break;
        }

        for handle in handles {
            if let Ok(Some((symbol, ratio))) = handle.join() {
                write!(output_file, "{}: {}\n", symbol, ratio).expect("Cant write to output file");
            }
            work_done += 1;
            println!("progress: {}%", work_done as f64 / all_work as f64 * 100.0);
        }
    }
}
