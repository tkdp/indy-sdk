use services::pool::types::LedgerStatus;
use services::pool::types::CatchupReq;
use services::pool::types::CatchupRep;
use services::pool::types::Message;
use services::pool::types::ConsistencyProof;
use services::pool::types::Reply;
use services::pool::types::Response;
use errors::common::CommonError;
use domain::ledger::constants;

use serde_json;
use serde_json::Value as SJsonValue;
use std::error::Error;

const REQUESTS_FOR_STATE_PROOFS: [&'static str; 7] = [
    constants::GET_NYM,
    constants::GET_SCHEMA,
    constants::GET_CRED_DEF,
    constants::GET_ATTR,
    constants::GET_REVOC_REG,
    constants::GET_REVOC_REG_DEF,
    constants::GET_REVOC_REG_DELTA,
];

const REQUEST_FOR_FULL: [&'static str; 2] = [
    constants::POOL_RESTART,
    constants::GET_VALIDATOR_INFO,
];

pub enum NetworkerEvent {
    SendOneRequest,
    SendAllRequest
}
#[derive(Clone)]
pub enum PoolEvent {
    CheckCache,
    NodeReply(
        String, // reply
        String, // node alias
    ),
    Close,
    Refresh,
    ConsensusReached,
    ConsensusFailed,
    PoolOutdated,
    Synced,
    NodesBlacklisted,
    SendRequest(
        String, // request
    ),
    Timeout
}

#[derive(Clone)]
pub enum RequestEvent {
    LedgerStatus(
        LedgerStatus
    ),
    CatchupReq(CatchupReq),
    CatchupRep(CatchupRep),
    CustomSingleRequest(
        String, // message
        Result<String, CommonError>, // req_id
    ),
    CustomConsensusRequest(
        String, // message
        Result<String, CommonError>, // req_id
    ),
    CustomFullRequest(
        String, // message
        Result<String, CommonError>, // req_id
    ),
    ConsistencyProof(ConsistencyProof),
    Reply(
        Reply,
        String, //raw_msg
        String, //node alias
        String, //req_id
    ),
    ReqACK(
        Response,
        String, //raw_msg
        String, //node alias
        String, //req_id
    ),
    ReqNACK(
        Response,
        String, //raw_msg
        String, //node alias
        String, //req_id
    ),
    Reject(
        Response,
        String, //raw_msg
        String, //node alias
        String, //req_id
    ),
    PoolLedgerTxns,
    Ping,
    Pong,
}

impl RequestEvent {
    pub fn get_req_id(&self) -> String {
        unimplemented!()
    }
}

impl From<(String, String, Message)> for RequestEvent {
    fn from((raw_msg, node_alias, msg): (String, String, Message)) -> Self {
        match msg {
            Message::CatchupReq(req) => RequestEvent::CatchupReq(req),
            Message::CatchupRep(rep) => RequestEvent::CatchupRep(rep),
            Message::LedgerStatus(ls) => RequestEvent::LedgerStatus(ls),
            Message::ConsistencyProof(cp) => RequestEvent::ConsistencyProof(cp),
            Message::Reply(rep) => {
                let req_id = rep.req_id();
                RequestEvent::Reply(rep, raw_msg, node_alias,req_id.to_string())
            }
            Message::ReqACK(rep) => {
                let req_id = rep.req_id();
                RequestEvent::ReqACK(rep, raw_msg, node_alias, req_id.to_string())
            }
            Message::ReqNACK(rep) => {
                let req_id = rep.req_id();
                RequestEvent::ReqNACK(rep, raw_msg, node_alias, req_id.to_string())
            },
            Message::Reject(rep) => {
                let req_id = rep.req_id();
                RequestEvent::Reject(rep, raw_msg, node_alias, req_id.to_string())
            },
            Message::PoolLedgerTxns(_) => RequestEvent::PoolLedgerTxns,
            Message::Ping => RequestEvent::Ping,
            Message::Pong => RequestEvent::Pong,
        }
    }
}

impl Into<Option<RequestEvent>> for PoolEvent {
    fn into(self) -> Option<RequestEvent> {
        match self {
            PoolEvent::NodeReply(msg, node_alias) => {
                _parse_msg(&msg).map(|parsed| (msg, node_alias, parsed).into())
            },
            PoolEvent::SendRequest(msg) => {
                let req_id = _parse_req_id_and_op(&msg);
                match req_id {
                    Ok((ref req_id, ref op)) if REQUESTS_FOR_STATE_PROOFS.contains(&op.as_str()) => Some(RequestEvent::CustomSingleRequest(msg, Ok(req_id.clone()))),
                    Ok((ref req_id, ref op)) if REQUEST_FOR_FULL.contains(&op.as_str()) => Some(RequestEvent::CustomFullRequest(msg, Ok(req_id.clone()))),
                    Ok((ref req_id, _)) => Some(RequestEvent::CustomConsensusRequest(msg, Ok(req_id.clone()))),
                    Err(err) => Some(RequestEvent::CustomSingleRequest(msg, Err(err))),
                }
            }
            _ => None
        }
    }
}


impl Into<Option<NetworkerEvent>> for RequestEvent {
    fn into(self) -> Option<NetworkerEvent> {
        match self {
            RequestEvent::LedgerStatus(_) => Some(NetworkerEvent::SendAllRequest),
            _ => None
        }
    }
}

fn _parse_msg(msg: &str) -> Option<Message> {
    match Message::from_raw_str(msg).map_err(map_err_trace!()) {
        Ok(msg) => Some(msg),
        Err(err) => None
    }
}

fn _parse_req_id_and_op(msg: &str) -> Result<(String, String), CommonError> {
    let req_json = _get_req_json(msg)?;

    let req_id: u64 = req_json["reqId"]
        .as_u64()
        .ok_or(CommonError::InvalidStructure("No reqId in request".to_string()))?;

    let op = req_json["operation"]["type"]
        .as_str()
        .ok_or(CommonError::InvalidStructure("No reqId in request".to_string()))?;

    Ok((req_id.to_string(), op.to_string()))
}

fn _get_req_json(msg: &str) -> Result<SJsonValue, CommonError> {
    serde_json::from_str(msg)
        .map_err(|err|
            CommonError::InvalidStructure(
                format!("Invalid request json: {}", err.description())))
}