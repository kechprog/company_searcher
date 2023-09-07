use lazy_static::lazy_static;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::thread;
use reqwest::blocking::Client;

mod company;
use company::Company;

lazy_static! {
    static ref CLIENT: Client = Client::new();
}

fn extract_keys<'a>(contents: &'a String) -> impl Iterator<Item = &'a str> {
    contents
        .split(", '")
        .flat_map(|item| item.split("': '").next())
        .map(|s| s.trim_matches(|c| c == '\'' || c == '{'))
}

fn filter (name: &str) -> Option<String> {
    let company = match Company::from_name_client(&name, &CLIENT) {
        Ok(company) => company,
        Err(_) => return None,
    };

    let net_debt = company.total_debt as f64 - company.free_cash as f64 + company.market_cap as f64 *1.45;

    let yr10_repayment = net_debt * ( (0.1) * 1.1f64.powf(120.0)
                                    / (1.1f64.powf(120.0) - 1.0) );
    
    let repayment_cash_flow = yr10_repayment * 12.0;

    let levered_cash_flow = company.free_cash_flow as f64 - repayment_cash_flow;

    let stress_level = 1.0 - (company.free_cash_flow as f64 - levered_cash_flow) / company.free_cash_flow as f64;

    if company.market_cap     < 100_000_000
    || company.market_cap     > 1_000_000_000
    || net_debt               > 1_000_000_000_f64
    || company.free_cash_flow < 0
    {
        return None;
    }

    let output = format!("{name},{market_cap},{total_debt},{free_cash},{free_cash_flow},{net_debt},{ratio},{yr10_repayment},{repayment_cash_flow},{levered_cash_flow},{stress_level}",
        name           = name,
        market_cap     = company.market_cap,
        total_debt     = company.total_debt,
        free_cash      = company.free_cash,
        free_cash_flow = company.free_cash_flow,
        net_debt       = net_debt,
        ratio          = net_debt as f64 / company.free_cash_flow as f64,
    );

    Some(output)
}

fn main() {
    let contents = fs::read_to_string("./symbols.txt").expect("Cant open/read file");
    let all_work = extract_keys(&contents).count();
    let mut work_done = 0_usize;
    let mut symbols = extract_keys(&contents);

    let mut output_file = OpenOptions::new()
        .append(true)
        .write(true)
        .create(true)
        .open("output.csv")
        .expect("Can't open output file");

    loop {
        let mut handles = vec![];
        while handles.len() < 30 {
            if let Some(symbol) = symbols.next() {
                let symbol = symbol.to_string();
                let handle = thread::spawn(move || filter(&symbol));
                handles.push(handle);
            } else {
                break;
            }
        }

        if handles.is_empty() {
            break;
        }
        
        if let Ok(Some(output)) = handles.pop().unwrap().join() {
            write!(output_file, "{}\n", output).expect("Cant write to output file");
        }

        work_done += 1;
        println!("progress: {}%", work_done as f64 / all_work as f64 * 100.0);
    }
}