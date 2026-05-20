use ethers::types::I256;
use serde::ser::SerializeTuple;
use serde_with::serde_as;
use serde_with::{base64::Base64, DisplayFromStr, NoneAsEmptyString};
use toner::ton::MsgAddress;

use crate::prelude::*;

pub type NumberOrString<T> = serde_with::PickFirst<(T, DisplayFromStr)>;

#[derive(Debug, Serialize, Deserialize)]
pub struct TonApiResult<T> {
    pub code: Option<i64>,
    pub error: Option<String>,
    pub ok: bool,
    pub result: Option<T>,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TransactionId {
    #[serde_as(as = "NumberOrString<_>")]
    pub lt: i64,
    #[serde_as(as = "Base64")]
    pub hash: [u8; 32],
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct FullAccountState {
    #[serde_as(as = "NumberOrString<_>")]
    pub balance: i64,
    #[serde_as(as = "Base64")]
    pub data: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub code: Vec<u8>,
    pub last_transaction_id: TransactionId,
    pub block_id: BlockIdExt,
    #[serde_as(as = "Base64")]
    pub frozen_hash: Vec<u8>,
    #[serde_as(as = "NumberOrString<_>")]
    pub sync_utime: i64,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct BlockIdExt {
    #[serde_as(as = "NumberOrString<_>")]
    pub workchain: i32,
    #[serde_as(as = "NumberOrString<_>")]
    pub shard: i64,
    #[serde_as(as = "NumberOrString<_>")]
    pub seqno: i32,
    #[serde_as(as = "Base64")]
    pub root_hash: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub file_hash: Vec<u8>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct AccountAddress {
    #[serde_as(as = "DisplayFromStr")]
    pub account_address: MsgAddress,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    pub address: AccountAddress,
    #[serde_as(as = "NumberOrString<_>")]
    pub utime: i64,
    #[serde_as(as = "Base64")]
    pub data: Vec<u8>,
    pub transaction_id: TransactionId,
    #[serde_as(as = "NumberOrString<_>")]
    pub fee: i64,
    #[serde_as(as = "NumberOrString<_>")]
    pub storage_fee: i64,
    #[serde_as(as = "NumberOrString<_>")]
    pub other_fee: i64,
    pub in_msg: Option<Message>,
    pub out_msgs: Vec<Message>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    #[serde_as(as = "DisplayFromStr")]
    pub source: MsgAddress,
    #[serde_as(as = "NoneAsEmptyString")]
    pub destination: Option<MsgAddress>,
    #[serde_as(as = "NumberOrString<_>")]
    pub value: i64,
    #[serde_as(as = "NumberOrString<_>")]
    pub fwd_fee: i64,
    #[serde_as(as = "NumberOrString<_>")]
    pub ihr_fee: i64,
    #[serde_as(as = "NumberOrString<_>")]
    pub created_lt: i64,
    #[serde_as(as = "Base64")]
    pub body_hash: Vec<u8>,
    pub msg_data: MessageData,
    pub message: Option<String>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "@type")]
pub enum MessageData {
    #[serde(rename = "msg.dataRaw")]
    Raw {
        #[serde_as(as = "Base64")]
        body: Vec<u8>,
        #[serde_as(as = "Base64")]
        init_state: Vec<u8>,
    },
    #[serde(rename = "msg.dataText")]
    Text {
        #[serde_as(as = "Base64")]
        text: Vec<u8>,
    },
    #[serde(rename = "msg.dataDecryptedText")]
    DecryptedText {
        #[serde_as(as = "Base64")]
        text: Vec<u8>,
    },
    #[serde(rename = "msg.dataEncryptedText")]
    EncryptedText {
        #[serde_as(as = "Base64")]
        text: Vec<u8>,
    },
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct RunResult {
    #[serde_as(as = "NumberOrString<_>")]
    pub gas_used: i64,
    pub stack: Vec<StackEntry>,
    pub exit_code: i64,
    pub block_id: BlockIdExt,
    pub last_transaction_id: TransactionId,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct RunGetMethod {
    #[serde_as(as = "DisplayFromStr")]
    pub address: MsgAddress,
    pub method: String,
    pub stack: Vec<StackEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub seqno: Option<i64>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct SendBoc {
    #[serde_as(as = "Base64")]
    pub boc: Vec<u8>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct SendBocResultHash {
    #[serde_as(as = "Base64")]
    pub hash: [u8; 32],
}

#[derive(Debug)]
pub enum StackEntry {
    Int(I256),
}

impl<'de> Deserialize<'de> for StackEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, value): (String, serde_json::Value) = Deserialize::deserialize(deserializer)?;

        match (tag.as_str(), value) {
            ("num", serde_json::Value::String(num)) => {
                let num = num.trim_start_matches("0x");
                if num.is_empty() || !num.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(serde::de::Error::custom("wrong integer"));
                }

                Ok(Self::Int(
                    I256::from_hex_str(num)
                        .map_err(|_| serde::de::Error::custom("wrong integer"))?,
                ))
            }
            _ => Err(serde::de::Error::custom("unexpected variant")),
        }
    }
}

impl Serialize for StackEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ser = serializer.serialize_tuple(2)?;
        match self {
            Self::Int(num) => {
                ser.serialize_element("num")?;
                ser.serialize_element(&format!("{:X}", num))?;
                ser.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

    fn valid_hash() -> String {
        base64::engine::general_purpose::STANDARD.encode([1_u8; 32])
    }

    fn valid_msg_address() -> String {
        format!("0:{}", "00".repeat(32))
    }

    fn valid_block_id() -> serde_json::Value {
        serde_json::json!({
            "workchain": "0",
            "shard": "0",
            "seqno": "1",
            "root_hash": "",
            "file_hash": "",
        })
    }

    fn valid_message() -> serde_json::Value {
        serde_json::json!({
            "source": valid_msg_address(),
            "destination": "",
            "value": "1",
            "fwd_fee": "0",
            "ihr_fee": "0",
            "created_lt": "1",
            "body_hash": "",
            "msg_data": {
                "@type": "msg.dataRaw",
                "body": "",
                "init_state": "",
            },
            "message": null,
        })
    }

    fn valid_transaction() -> serde_json::Value {
        serde_json::json!({
            "address": {
                "account_address": valid_msg_address(),
            },
            "utime": "1",
            "data": "",
            "transaction_id": {
                "lt": "1",
                "hash": valid_hash(),
            },
            "fee": "0",
            "storage_fee": "0",
            "other_fee": "0",
            "in_msg": null,
            "out_msgs": [],
        })
    }

    #[test]
    fn stack_entry_rejects_unexpected_tag() {
        let err = serde_json::from_value::<StackEntry>(serde_json::json!(["cell", "0x1"]))
            .expect_err("unexpected stack entry tag must fail");

        assert!(err.to_string().contains("unexpected variant"));
    }

    #[test]
    fn stack_entry_rejects_non_string_integer_payload() {
        let err = serde_json::from_value::<StackEntry>(serde_json::json!(["num", 1]))
            .expect_err("non-string integer payload must fail");

        assert!(err.to_string().contains("unexpected variant"));
    }

    #[test]
    fn stack_entry_rejects_invalid_hex_integer() {
        let err = serde_json::from_value::<StackEntry>(serde_json::json!(["num", "0xnot-hex"]))
            .expect_err("invalid hex integer must fail");

        assert!(err.to_string().contains("wrong integer"));
    }

    #[test]
    fn stack_entry_rejects_empty_hex_integer() {
        for value in [
            serde_json::json!(["num", ""]),
            serde_json::json!(["num", "0x"]),
        ] {
            assert!(
                serde_json::from_value::<StackEntry>(value).is_err(),
                "empty hex integer must fail"
            );
        }
    }

    #[test]
    fn stack_entry_rejects_signed_or_whitespace_hex_integer() {
        for value in [" 1", "1 ", "\t1", "0x 1", "0x_1", "+1", "--1"] {
            assert!(
                serde_json::from_value::<StackEntry>(serde_json::json!(["num", value])).is_err(),
                "ambiguous hex integer {value:?} must fail"
            );
        }
    }

    #[test]
    fn stack_entry_rejects_malformed_tuple_shape() {
        for value in [
            serde_json::json!(["num"]),
            serde_json::json!(["num", "0x1", "extra"]),
            serde_json::json!({"num": "0x1"}),
        ] {
            assert!(
                serde_json::from_value::<StackEntry>(value).is_err(),
                "malformed stack entry shape must fail"
            );
        }
    }

    #[test]
    fn transaction_id_rejects_invalid_hash_encoding() {
        for value in [
            serde_json::json!({
                "lt": "1",
                "hash": "not-base64",
            }),
            serde_json::json!({
                "lt": "1",
                "hash": base64::engine::general_purpose::STANDARD.encode([1_u8; 31]),
            }),
            serde_json::json!({
                "lt": "1",
                "hash": base64::engine::general_purpose::STANDARD.encode([1_u8; 33]),
            }),
        ] {
            assert!(
                serde_json::from_value::<TransactionId>(value).is_err(),
                "invalid TON transaction hash must fail"
            );
        }
    }

    #[test]
    fn transaction_id_rejects_missing_required_fields() {
        for value in [
            serde_json::json!({ "hash": valid_hash() }),
            serde_json::json!({ "lt": "1" }),
            serde_json::json!({}),
        ] {
            assert!(
                serde_json::from_value::<TransactionId>(value).is_err(),
                "transaction id missing required fields must fail"
            );
        }
    }

    #[test]
    fn transaction_id_rejects_invalid_logical_time() {
        let value = serde_json::json!({
            "lt": "not-a-number",
            "hash": valid_hash(),
        });

        assert!(serde_json::from_value::<TransactionId>(value).is_err());
    }

    #[test]
    fn transaction_id_rejects_out_of_range_logical_time() {
        for lt in [
            "9223372036854775808",
            "-9223372036854775809",
            "18446744073709551616",
        ] {
            let value = serde_json::json!({
                "lt": lt,
                "hash": valid_hash(),
            });

            assert!(
                serde_json::from_value::<TransactionId>(value).is_err(),
                "out-of-range logical time {lt} must fail"
            );
        }
    }

    #[test]
    fn transaction_id_rejects_non_integer_json_types() {
        for lt in [
            serde_json::json!(1.25),
            serde_json::json!(true),
            serde_json::json!(null),
            serde_json::json!({ "nested": 1 }),
        ] {
            let value = serde_json::json!({
                "lt": lt,
                "hash": valid_hash(),
            });

            assert!(
                serde_json::from_value::<TransactionId>(value).is_err(),
                "non-integer logical time must fail"
            );
        }
    }

    #[test]
    fn ton_api_result_rejects_malformed_present_result_even_when_not_ok() {
        let value = serde_json::json!({
            "ok": false,
            "code": 500,
            "error": "upstream error",
            "result": {
                "lt": "1",
                "hash": "not-base64"
            }
        });

        assert!(serde_json::from_value::<TonApiResult<TransactionId>>(value).is_err());
    }

    #[test]
    fn ton_api_result_rejects_result_with_wrong_nested_type_even_when_erroring() {
        let value = serde_json::json!({
            "ok": false,
            "code": 400,
            "error": "bad request",
            "result": {
                "gas_used": "1",
                "stack": [["num", "0x"]],
                "exit_code": 0,
                "block_id": valid_block_id(),
                "last_transaction_id": {
                    "lt": "1",
                    "hash": valid_hash(),
                },
            },
        });

        assert!(serde_json::from_value::<TonApiResult<RunResult>>(value).is_err());
    }

    #[test]
    fn ton_api_result_rejects_wrong_envelope_types() {
        for value in [
            serde_json::json!({
                "ok": "true",
                "code": null,
                "error": null,
                "result": null,
            }),
            serde_json::json!({
                "ok": false,
                "code": "500",
                "error": "upstream",
                "result": null,
            }),
            serde_json::json!({
                "ok": false,
                "code": 500,
                "error": { "message": "upstream" },
                "result": null,
            }),
        ] {
            assert!(
                serde_json::from_value::<TonApiResult<TransactionId>>(value).is_err(),
                "malformed TON API envelope must fail"
            );
        }
    }

    #[test]
    fn ton_api_result_rejects_missing_ok_flag() {
        let value = serde_json::json!({
            "code": null,
            "error": null,
            "result": null,
        });

        assert!(serde_json::from_value::<TonApiResult<TransactionId>>(value).is_err());
    }

    #[test]
    fn ton_api_result_rejects_wrong_result_shape_when_ok() {
        let value = serde_json::json!({
            "ok": true,
            "code": null,
            "error": null,
            "result": "not-a-transaction-id",
        });

        assert!(serde_json::from_value::<TonApiResult<TransactionId>>(value).is_err());
    }

    #[test]
    fn message_rejects_invalid_source_or_destination_addresses() {
        for (field, value) in [
            ("source", serde_json::json!("not-an-address")),
            ("source", serde_json::json!("0:abcd")),
            ("destination", serde_json::json!("not-an-address")),
            ("destination", serde_json::json!("0:abcd")),
        ] {
            let mut message = valid_message();
            message
                .as_object_mut()
                .expect("message should be object")
                .insert(field.to_string(), value);

            assert!(
                serde_json::from_value::<Message>(message).is_err(),
                "message with invalid {field} must fail"
            );
        }
    }

    #[test]
    fn message_rejects_non_integer_amount_and_lt_fields() {
        for (field, value) in [
            ("value", serde_json::json!(1.25)),
            ("fwd_fee", serde_json::json!(true)),
            ("ihr_fee", serde_json::json!({ "fee": 0 })),
            ("created_lt", serde_json::json!("not-a-number")),
        ] {
            let mut message = valid_message();
            message
                .as_object_mut()
                .expect("message should be object")
                .insert(field.to_string(), value);

            assert!(
                serde_json::from_value::<Message>(message).is_err(),
                "message with malformed {field} must fail"
            );
        }
    }

    #[test]
    fn message_data_rejects_unknown_type() {
        let value = serde_json::json!({
            "@type": "msg.dataExecutable",
            "body": "",
            "init_state": ""
        });

        assert!(serde_json::from_value::<MessageData>(value).is_err());
    }

    #[test]
    fn message_data_rejects_missing_type_tag() {
        for value in [
            serde_json::json!({
                "body": "",
                "init_state": "",
            }),
            serde_json::json!({
                "text": "",
            }),
        ] {
            assert!(
                serde_json::from_value::<MessageData>(value).is_err(),
                "message data without @type must fail"
            );
        }
    }

    #[test]
    fn message_data_rejects_invalid_base64_payloads() {
        for value in [
            serde_json::json!({
                "@type": "msg.dataRaw",
                "body": "not-base64",
                "init_state": ""
            }),
            serde_json::json!({
                "@type": "msg.dataText",
                "text": "not-base64"
            }),
            serde_json::json!({
                "@type": "msg.dataEncryptedText",
                "text": "not-base64"
            }),
            serde_json::json!({
                "@type": "msg.dataDecryptedText",
                "text": "not-base64"
            }),
        ] {
            assert!(
                serde_json::from_value::<MessageData>(value).is_err(),
                "invalid base64 message data must fail"
            );
        }
    }

    #[test]
    fn message_data_rejects_missing_required_payload_fields() {
        for value in [
            serde_json::json!({
                "@type": "msg.dataRaw",
                "body": "",
            }),
            serde_json::json!({
                "@type": "msg.dataText",
            }),
            serde_json::json!({
                "@type": "msg.dataEncryptedText",
            }),
            serde_json::json!({
                "@type": "msg.dataDecryptedText",
            }),
        ] {
            assert!(
                serde_json::from_value::<MessageData>(value).is_err(),
                "missing message data payload field must fail"
            );
        }
    }

    #[test]
    fn send_boc_rejects_invalid_base64_body() {
        let value = serde_json::json!({ "boc": "not-base64" });

        assert!(serde_json::from_value::<SendBoc>(value).is_err());
    }

    #[test]
    fn send_boc_rejects_missing_boc_body() {
        assert!(serde_json::from_value::<SendBoc>(serde_json::json!({})).is_err());
    }

    #[test]
    fn block_id_rejects_invalid_numeric_fields() {
        for (field, value) in [
            ("workchain", serde_json::json!("not-a-number")),
            ("shard", serde_json::json!(1.25)),
            ("seqno", serde_json::json!(true)),
        ] {
            let mut block_id = valid_block_id();
            block_id
                .as_object_mut()
                .expect("block id should be object")
                .insert(field.to_string(), value);

            assert!(
                serde_json::from_value::<BlockIdExt>(block_id).is_err(),
                "block id with invalid {field} must fail"
            );
        }
    }

    #[test]
    fn block_id_rejects_invalid_hash_encodings() {
        for field in ["root_hash", "file_hash"] {
            let mut block_id = valid_block_id();
            block_id
                .as_object_mut()
                .expect("block id should be object")
                .insert(field.to_string(), serde_json::json!("not-base64"));

            assert!(
                serde_json::from_value::<BlockIdExt>(block_id).is_err(),
                "block id with invalid {field} must fail"
            );
        }
    }

    #[test]
    fn send_boc_result_hash_rejects_wrong_length_hashes() {
        for hash in [
            base64::engine::general_purpose::STANDARD.encode([1_u8; 0]),
            base64::engine::general_purpose::STANDARD.encode([1_u8; 31]),
            base64::engine::general_purpose::STANDARD.encode([1_u8; 33]),
        ] {
            let value = serde_json::json!({ "hash": hash });

            assert!(
                serde_json::from_value::<SendBocResultHash>(value).is_err(),
                "wrong-length sendBoc hash must fail"
            );
        }
    }

    #[test]
    fn send_boc_result_hash_rejects_missing_hash() {
        assert!(serde_json::from_value::<SendBocResultHash>(serde_json::json!({})).is_err());
    }

    #[test]
    fn run_get_method_rejects_invalid_address_stack_and_seqno() {
        for (field, value) in [
            ("address", serde_json::json!("not-an-address")),
            ("stack", serde_json::json!([["num", "0x"]])),
            ("seqno", serde_json::json!("not-a-number")),
            ("seqno", serde_json::json!(1.25)),
        ] {
            let mut request = serde_json::json!({
                "address": valid_msg_address(),
                "method": "seqno",
                "stack": [],
                "seqno": 1,
            });
            request
                .as_object_mut()
                .expect("run get method should be object")
                .insert(field.to_string(), value);

            assert!(
                serde_json::from_value::<RunGetMethod>(request).is_err(),
                "run get method with malformed {field} must fail"
            );
        }
    }

    #[test]
    fn run_result_rejects_missing_required_fields() {
        for field in [
            "gas_used",
            "stack",
            "exit_code",
            "block_id",
            "last_transaction_id",
        ] {
            let mut value = serde_json::json!({
                "gas_used": "1",
                "stack": [["num", "0x1"]],
                "exit_code": 0,
                "block_id": valid_block_id(),
                "last_transaction_id": {
                    "lt": "1",
                    "hash": valid_hash(),
                },
            });
            value
                .as_object_mut()
                .expect("run result should be object")
                .remove(field);

            assert!(
                serde_json::from_value::<RunResult>(value).is_err(),
                "run result missing {field} must fail"
            );
        }
    }

    #[test]
    fn run_result_rejects_malformed_stack_entry() {
        let value = serde_json::json!({
            "gas_used": "1",
            "stack": [["cell", "0x1"]],
            "exit_code": 0,
            "block_id": valid_block_id(),
            "last_transaction_id": {
                "lt": "1",
                "hash": valid_hash(),
            },
        });

        assert!(serde_json::from_value::<RunResult>(value).is_err());
    }

    #[test]
    fn transaction_rejects_invalid_nested_address_and_messages() {
        for (field, value) in [
            (
                "address",
                serde_json::json!({ "account_address": "not-an-address" }),
            ),
            (
                "out_msgs",
                serde_json::json!([valid_message(), ["not-a-message"]]),
            ),
            ("in_msg", serde_json::json!(["not-a-message"])),
        ] {
            let mut transaction = valid_transaction();
            transaction
                .as_object_mut()
                .expect("transaction should be object")
                .insert(field.to_string(), value);

            assert!(
                serde_json::from_value::<Transaction>(transaction).is_err(),
                "transaction with malformed {field} must fail"
            );
        }
    }
}
