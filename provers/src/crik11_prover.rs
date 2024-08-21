use actix_web::{web, HttpResponse, Responder};
use http_body_util::Empty;
use hyper::{body::Bytes, Request, StatusCode};
use hyper_util::rt::TokioIo;
use std::ops::Range;
use tlsn_core::proof::TlsProof;
use tokio::io::AsyncWriteExt;
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};

use tlsn_prover::tls::{Prover, ProverConfig};
use crate::setup_notary_connection;

// Setting of the application server
const SERVER_DOMAIN: &str = "crik-match.vercel.app";
const ROUTE: &str = "api/match-info";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36";

// Setting of the notary server
const NOTARY_HOST: &str = "127.0.0.1";
const NOTARY_PORT: u16 = 7047;

// Configuration of notarization
const NOTARY_MAX_TRANSCRIPT_SIZE: usize = 16384;

#[derive(serde::Deserialize)]
pub struct Crik11QueryParams {
   match_id: String,
}

pub async fn notarize(query_params: web::Query<Crik11QueryParams>) -> impl Responder {
    let match_id = &query_params.match_id;

    println!("Connected to the Notary");

    let (notary_socket, session_id) =
        setup_notary_connection(NOTARY_HOST, NOTARY_PORT, Some(NOTARY_MAX_TRANSCRIPT_SIZE)).await;
    println!("session ID: {}", session_id);
    let config = ProverConfig::builder()
        .id(session_id)
        .server_dns(SERVER_DOMAIN)
        .build()
        .unwrap();

    let prover = Prover::new(config)
        .setup(notary_socket.compat())
        .await
        .unwrap();

    let client_socket = tokio::net::TcpStream::connect((SERVER_DOMAIN, 443))
        .await
        .unwrap();

    let (mpc_tls_connection, prover_fut) = prover.connect(client_socket.compat()).await.unwrap();

    let prover_task = tokio::spawn(prover_fut);

    let (mut request_sender, connection) =
        hyper::client::conn::http1::handshake(TokioIo::new(mpc_tls_connection.compat()))
            .await
            .unwrap();

    let connection_task = tokio::spawn(connection.without_shutdown());

    let formatted_uri = format!(
        "https://{}/{}?match_id={}",
        SERVER_DOMAIN, ROUTE, match_id
    );

    println!("Sending request to the server");
    let request = Request::builder()
        .uri(formatted_uri)
        .header("Host", SERVER_DOMAIN)
        .header("User-Agent", "zkNotary")
        .header("Accept", "*/*")
        .body(Empty::<Bytes>::new())
        .unwrap();
    println!("Starting an MPC TLS connection with the server");

    let response = request_sender.send_request(request).await.unwrap();

    println!("Got a response from the server");

    assert!(response.status() == StatusCode::OK);

    let mut client_socket = connection_task.await.unwrap().unwrap().io.into_inner();
    client_socket.shutdown().await.unwrap();

    let prover = prover_task.await.unwrap().unwrap();

    let mut prover = prover.start_notarize();
        
    let (sent_public_ranges, _) = find_ranges(
        prover.sent_transcript().data(),
        &[USER_AGENT.as_bytes()],
    );
    println!("Sent public ranges: {:?}", sent_public_ranges);

    let (recv_public_ranges, _) = find_ranges(
        prover.recv_transcript().data(),
        &[],
    );

    let builder = prover.commitment_builder();
    println!("Received public ranges: {:?}", recv_public_ranges);

    let sent_commitments: Vec<_> = sent_public_ranges
        .iter()
        .map(|r| builder.commit_sent(r).unwrap())
        .collect();

    let recv_commitments: Vec<_> = recv_public_ranges
        .iter()
        .map(|r| builder.commit_recv(r).unwrap())
        .collect();

    let notarized_session = prover.finalize().await.unwrap();

    let mut proof_builder = notarized_session.data().build_substrings_proof();

    for commitment_id in sent_commitments {
        proof_builder.reveal_by_id(commitment_id).unwrap();
    }
    for commitment_id in recv_commitments {
        proof_builder.reveal_by_id(commitment_id).unwrap();
    }

    let substrings_proof = proof_builder.build().unwrap();

    let proof = TlsProof {
        session: notarized_session.session_proof(),
        substrings: substrings_proof,
    };

    println!("Notarization completed successfully!");

    HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string_pretty(&proof).unwrap())
}

fn find_ranges(seq: &[u8], private_seq: &[&[u8]]) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let mut private_ranges = Vec::new();
    for s in private_seq {
        for (idx, w) in seq.windows(s.len()).enumerate() {
            if w == *s {
                private_ranges.push(idx..(idx + w.len()));
            }
        }
    }

    let mut sorted_ranges = private_ranges.clone();
    sorted_ranges.sort_by_key(|r| r.start);

    let mut public_ranges = Vec::new();
    let mut last_end = 0;
    for r in sorted_ranges {
        if r.start > last_end {
            public_ranges.push(last_end..r.start);
        }
        last_end = r.end;
    }

    if last_end < seq.len() {
        public_ranges.push(last_end..seq.len());
    }

    (public_ranges, private_ranges)
}