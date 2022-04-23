use comfy_table::{Cell, Table};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use structopt::StructOpt;
// use trust_dns_resolver::config::*;
// use trust_dns_resolver::Resolver;

#[derive(StructOpt, Debug)]
enum Operation {
    Scan {
        #[structopt(short, long)]
        host: String,
    },
    List,
    Add {
        #[structopt(short, long)]
        application: String,
        #[structopt(short, long)]
        ports: String,
    },
    Delete,
}

fn parse_ports(ports: String) -> Vec<u16> {
    let mut ports_vec: Vec<u16> = Vec::new();
    let ports: Vec<&str> = ports.split(',').collect();
    for port in ports {
        if port.contains('-') {
            let port_range: Vec<&str> = port.split('-').collect();
            let limit_inf: u16 = port_range[0].parse().unwrap();
            let limit_sup: u16 = port_range[1].parse().unwrap();
            for p in limit_inf..=limit_sup {
                ports_vec.push(p);
            }
        } else {
            ports_vec.push(port.parse::<u16>().unwrap());
        }
    }

    ports_vec
}

fn list(records_vec: &Vec<Record>) {
    let mut table = Table::new();
    table.set_header(vec!["#", "App Name", "Ports"]);
    let mut id: u16 = 1;
    for record in records_vec {
        let mut ports_string: String = "".to_owned();
        for p in &record.ports {
            ports_string.push_str(p.to_string().as_str());
            ports_string.push_str(" ");
        }
        table.add_row(vec![
            Cell::new(id.to_string()),
            Cell::new(String::from(&record.name)),
            Cell::new(ports_string),
        ]);
        id += 1;
    }
    println!("{}", table);
}

fn add(name: String, ports: String, records_vec: &mut Vec<Record>) -> io::Result<()> {
    let ports = parse_ports(ports);
    let new_record: Record = Record { name, ports };
    records_vec.push(new_record);
    let file = OpenOptions::new().write(true).open("apps-ports")?;
    serde_json::to_writer(&file, &records_vec)?;
    Ok(())
}

fn delete(records_vec: &mut Vec<Record>) -> io::Result<()> {
    list(&records_vec);
    println!("before: {:?}", records_vec);
    println!("Enter number of app to delete");
    let choice = user_input().unwrap();
    let choice: usize = choice.trim().parse().unwrap();
    // let records_vec: Vec<Record> = records_vec[0..choice - 1].concat(records_vec[choice + 1..]);
    records_vec.swap_remove(choice - 1);

    println!("after: {:?}", records_vec);
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open("apps-ports")?;
    serde_json::to_writer(&file, &records_vec)?;
    Ok(())
}

fn parse_host(host: String) -> Result<Ipv4Addr, String> {
    if let Ok(ip) = Ipv4Addr::from_str(&host) {
        return Ok(ip);
    }
    Err(String::from("Must provide a valid ip address"))
}

fn scan(host: String, records_vec: Vec<Record>) -> Result<(), String> {
    let host: Ipv4Addr = parse_host(host)?;
    let host: IpAddr = IpAddr::V4(host);
    list(&records_vec);
    println!("Enter number of app to scan");
    let choice = user_input().unwrap();
    let choice: usize = choice.trim().parse().unwrap();
    if choice > records_vec.len() {
        return Err(format!(
            "Selection should be betwenn 1 and {:?}.",
            records_vec.len()
        ));
    }
    for port in &records_vec[choice - 1].ports {
        // let socket = SocketAddrV4::new(host, port.to_owned());
        let socket: SocketAddr = SocketAddr::new(host, port.to_owned());
        match TcpStream::connect_timeout(&socket, Duration::new(5, 0)) {
            Ok(_) => println!("\t[*] Port {} is open", port),
            Err(error) => println!("\t[x] Port {} not connected: {:?}", port, error.kind()),
        }
    }
    Ok(())
}

fn user_input() -> io::Result<String> {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    Ok(buffer)
}

#[derive(Debug, Serialize, Deserialize)]
struct Record {
    name: String,
    ports: Vec<u16>,
}

fn main() {
    let opt = Operation::from_args();

    let path = Path::new("apps-ports");
    let display = path.display();

    if !path.exists() {
        match File::create(&path) {
            Err(why) => panic!("couldn't create {}: {}", display, why),
            Ok(_) => {
                let file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open("apps-ports")
                    .unwrap();
                let new_record: Vec<Record> = vec![Record {
                    name: "Office".to_owned(),
                    ports: vec![1688],
                }];
                serde_json::to_writer(&file, &new_record).unwrap();
            }
        };
    }

    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);

    let mut records_vec: Vec<Record> = serde_json::from_reader(reader).unwrap();

    match opt {
        Operation::Add { application, ports } => match add(application, ports, &mut records_vec) {
            Ok(_) => println!("Record added."),
            Err(e) => println!("Error while adding the record: {}", e),
        },
        Operation::Scan { host } => match scan(host, records_vec) {
            Ok(_) => (),
            Err(e) => println!("Error: {:?}", e),
        },
        Operation::List => list(&records_vec),
        Operation::Delete => match delete(&mut records_vec) {
            Ok(_) => println!("Record deleted,"),
            Err(e) => println!("Error while deleting: {}.", e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_range_ports() {
        let input: String = "27000-27009".to_owned();
        let expected: Vec<u16> = vec![
            27000, 27001, 27002, 27003, 27004, 27005, 27006, 27007, 27008, 27009,
        ];
        assert_eq!(expected, parse_ports(input));
    }
    #[test]
    fn parse_comma_separated_ports() {
        let input: String = "123,456,789".to_owned();
        let expected: Vec<u16> = vec![123, 456, 789];
        assert_eq!(expected, parse_ports(input));
    }
    #[test]
    fn parse_mixed_range_comma_separated_ports() {
        let input: String = "1688,10-15".to_owned();
        let expected: Vec<u16> = vec![1688, 10, 11, 12, 13, 14, 15];
        assert_eq!(expected, parse_ports(input));
    }
}
