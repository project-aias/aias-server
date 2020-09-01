use actix_web::{HttpResponse, Responder, web};
use fair_blind_signature::CheckParameter;

use aias_core::signer::Signer;
use std::sync::{Arc, Mutex};

use crate::utils::{self, Keys};

pub async fn hello() -> impl Responder {
    println!("hello");
    
    HttpResponse::Ok().body("Hello world")
}


pub async fn ready(body: web::Bytes, actix_data: web::Data<Arc<Mutex<Keys>>>) -> Result<String, HttpResponse> {
    println!("ready");

    let signer_privkey = actix_data.lock().unwrap().signer_privkey.clone();
    let signer_pubkey = actix_data.lock().unwrap().signer_pubkey.clone();

    let judge_pubkey = actix_data.lock().unwrap().judge_pubkey.clone();

    let body = String::from_utf8_lossy(&body).to_string();
    let mut signer = Signer::new_with_blinded_digest(signer_privkey, signer_pubkey, body);

    let subset = signer.setup_subset();

    let conn = utils::db_connection();

    conn.execute("INSERT INTO sign_process (phone, m, subset)
                  VALUES ($1, $2, $3)",
                 &["10", &body.to_string(), &subset]).unwrap();

    Ok(subset)
}


pub async fn sign(body: web::Bytes, actix_data: web::Data<Arc<Mutex<Keys>>>) -> Result<String, HttpResponse> {
    println!("sign");

    let id = 10;

    let signer_privkey = actix_data.lock().unwrap().signer_privkey.clone();
    let signer_pubkey = actix_data.lock().unwrap().signer_pubkey.clone();
    let judge_pubkey = actix_data.lock().unwrap().judge_pubkey.clone();


    let conn = utils::db_connection();
    let mut stmt = conn.prepare("SELECT m, subset FROM sign_process WHERE phone=?")
        .expect("failed to select");

    struct SubsetData {
        m: String,
        subset: String
    }

    let SubsetData {m, subset} = stmt.query_row(rusqlite::params!["10"], |row| {
        Ok(SubsetData {
            m: row.get(0).unwrap(),
            subset: row.get(1).unwrap()
        })
    })
    .unwrap();

    let mut signer = Signer::new_from_params(signer_privkey, signer_pubkey, judge_pubkey, m, subset);

    let body = body.to_vec();
    let check_parameter = String::from_utf8_lossy(&body);

    if !signer.check(check_parameter.to_string()) {
        return Err(utils::bad_request("invalid"))
    }

    let signature = signer.sign();

    Ok(signature)
}
