use std::{io::{copy, Read, Result, Write}, net::{TcpListener, TcpStream}, thread};

const SOCKS_VERSION: u8 = 0x05;

fn main(){
    let listener = TcpListener::bind("127.0.0.1:3000").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // Handle new client connection
                match client(stream) {
                    Ok(_) => (),
                    Err(e) => println!("Failed to handle client: {}",e)
                }
            }
            Err(e) => { 
                // Handle connection error
                println!("Failed to handle incoming connection: {}",e)
            }
        }
    }
}

fn client(mut client_stream:TcpStream) -> Result<()> {
    // greeting header
    let mut buffer: [u8; 2] = [0; 2];
    client_stream.read(&mut buffer[..])?;
    let _version = buffer[0]; // should be the same as SOCKS_VERSION
    let number_of_methods = buffer[1];

    // authentication methods
    let mut methods: Vec<u8> = vec![];
    for _ in 0..number_of_methods {
        let mut next_method: [u8; 1] = [0; 1];
        client_stream.read(&mut next_method[..])?;
        methods.push(next_method[0]);
    }

    // only accept no authentication
    if !methods.contains(&0x00) {
        // no acceptable methods were offered
        client_stream.write(&[SOCKS_VERSION, 0xFF])?;
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Authentication method not supported"));
    }

    // we choose no authentication
    client_stream.write(&[SOCKS_VERSION, 0x00])?;

    // read connection request
    //request  [VER, CMD, RSV, ATYP, ADDR, PORT]
    //response [VER, RES, RSV, ATYP, ADDR, PORT]
    let mut buffer: [u8; 100] = [0; 100];
    client_stream.read(&mut buffer)?;

    let request_str: String = core::str::from_utf8(&buffer).unwrap().to_owned();

    println!("Client request: {:#?}", request_str);

    let mut address: String = request_str[4..request_str.len()-2].to_string();
    address.push_str(":");
    address.push_str(&request_str[request_str.len()-3..]);

    let mut remote_stream = TcpStream::connect(address).unwrap();

    //change second buffer index from command to responsecode so it can be used as response
    buffer[1] = 0x00;

    client_stream.write(&buffer)?;

    // clone our streams
    let mut incoming_local = client_stream.try_clone()?;
    let mut incoming_remote = remote_stream.try_clone()?;

    // copy the data from one to the other
    let handle_outgoing = thread::spawn(move || -> std::io::Result<()> {
        copy(&mut client_stream, &mut remote_stream)?;
        Ok(())
    });
    let handle_incoming = thread::spawn(move || -> std::io::Result<()> {
        copy(&mut incoming_remote, &mut incoming_local)?;
        Ok(())
    });

    // if we get any errors now its not our problem
    _ = handle_outgoing.join();
    _ = handle_incoming.join();

    Ok(())
}