#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    instantiate2_address, to_json_binary, Binary, CodeInfoResponse, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, WasmMsg,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Dataverse, DATAVERSE};

// version info for migration info
const CONTRACT_NAME: &str = concat!("crates.io:", env!("CARGO_PKG_NAME"));
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
    let CodeInfoResponse { checksum, .. } = deps
        .querier
        .query_wasm_code_info(msg.triplestore_config.code_id.u64())?;
    let salt = Binary::from(msg.name.as_bytes());

    let _triplestore_address = instantiate2_address(&checksum, &creator, &salt)?;

    // Necessary stuff for testing purposes, see: https://github.com/CosmWasm/cosmwasm/issues/1648
    let triplestore_address = {
        #[cfg(not(test))]
        {
            deps.api.addr_humanize(&_triplestore_address)?
        }
        #[cfg(test)]
        cosmwasm_std::Addr::unchecked("predicted address")
    };

    DATAVERSE.save(
        deps.storage,
        &Dataverse {
            name: msg.name.clone(),
            triplestore_address: triplestore_address.clone(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("triplestore_address", triplestore_address.to_string())
        .add_message(WasmMsg::Instantiate2 {
            admin: Some(env.contract.address.to_string()),
            code_id: msg.triplestore_config.code_id.u64(),
            label: format!("{}_triplestore", msg.name),
            msg: to_json_binary(&okp4_cognitarium::msg::InstantiateMsg {
                limits: msg.triplestore_config.limits.into(),
            })?,
            funds: vec![],
            salt,
        }))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SubmitClaims {
            metadata,
            format: _,
        } => execute::submit_claims(deps, info, metadata),
        _ => Err(StdError::generic_err("Not implemented").into()),
    }
}

pub mod execute {
    use super::*;
    use crate::credential::vc::VerifiableCredential;
    use crate::registrar::credential::DataverseCredential;
    use crate::registrar::registry::ClaimRegistrar;
    use okp4_rdf::dataset::Dataset;
    use okp4_rdf::serde::NQuadsReader;
    use std::io::BufReader;

    pub fn submit_claims(
        deps: DepsMut<'_>,
        info: MessageInfo,
        data: Binary,
    ) -> Result<Response, ContractError> {
        let buf = BufReader::new(data.as_slice());
        let mut reader = NQuadsReader::new(buf);
        let rdf_quads = reader.read_all()?;
        let vc_dataset = Dataset::from(rdf_quads.as_slice());
        let vc = VerifiableCredential::try_from(&vc_dataset)?;
        vc.verify(&deps)?;

        let credential = DataverseCredential::try_from((info.sender, &vc))?;
        let registrar = ClaimRegistrar::try_new(deps.storage)?;

        Ok(Response::default()
            .add_attribute("action", "submit_claims")
            .add_attribute("credential", credential.id)
            .add_attribute("subject", credential.subject)
            .add_attribute("type", credential.r#type)
            .add_message(registrar.submit_claim(&deps, &credential)?))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps<'_>, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Err(StdError::generic_err("Not implemented"))
}

pub mod query {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::{RdfFormat, TripleStoreConfig, TripleStoreLimitsInput};
    use crate::testutil::testutil::read_test_data;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{
        from_json, Addr, Attribute, ContractResult, CosmosMsg, HexBinary, SubMsg, SystemError,
        SystemResult, Uint128, Uint64, WasmQuery,
    };
    use okp4_cognitarium::msg::{
        DataFormat, Head, Node, Results, SelectItem, SelectQuery, SelectResponse,
        SimpleWhereCondition, TriplePattern, Value, VarOrNode, VarOrNodeOrLiteral, WhereCondition,
        IRI,
    };
    use std::collections::BTreeMap;

    #[test]
    fn proper_instantiate() {
        let mut deps = mock_dependencies();
        deps.querier.update_wasm(|query| match query {
            WasmQuery::CodeInfo { code_id, .. } => {
                let resp = CodeInfoResponse::new(
                    code_id.clone(),
                    "creator".to_string(),
                    HexBinary::from_hex(
                        "3B94AAF0B7D804B5B458DED0D20CACF95D2A1C8DF78ED3C89B61291760454AEC",
                    )
                    .unwrap(),
                );
                SystemResult::Ok(ContractResult::Ok(to_json_binary(&resp).unwrap()))
            }
            _ => SystemResult::Err(SystemError::Unknown {}),
        });

        let store_limits = TripleStoreLimitsInput {
            max_byte_size: Some(Uint128::from(50000u128)),
            ..Default::default()
        };

        let msg = InstantiateMsg {
            name: "my-dataverse".to_string(),
            triplestore_config: TripleStoreConfig {
                code_id: Uint64::from(17u64),
                limits: store_limits.clone(),
            },
        };

        let env = mock_env();
        let res = instantiate(deps.as_mut(), env.clone(), mock_info("creator", &[]), msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![Attribute::new("triplestore_address", "predicted address")]
        );
        assert_eq!(
            res.messages,
            vec![SubMsg::new(WasmMsg::Instantiate2 {
                admin: Some(env.contract.address.to_string()),
                code_id: 17,
                label: "my-dataverse_triplestore".to_string(),
                msg: to_json_binary(&okp4_cognitarium::msg::InstantiateMsg {
                    limits: store_limits.into(),
                })
                .unwrap(),
                funds: vec![],
                salt: Binary::from("my-dataverse".as_bytes()),
            })]
        );
        assert_eq!(
            DATAVERSE.load(&deps.storage).unwrap(),
            Dataverse {
                name: "my-dataverse".to_string(),
                triplestore_address: Addr::unchecked("predicted address"),
            }
        )
    }

    #[test]
    fn proper_submit_claims() {
        let mut deps = mock_dependencies();
        deps.querier.update_wasm(|query| match query {
            WasmQuery::Smart { contract_addr, msg } => {
                if contract_addr != "my-dataverse-addr" {
                    return SystemResult::Err(SystemError::NoSuchContract {
                        addr: contract_addr.to_string(),
                    });
                }
                let query_msg: StdResult<okp4_cognitarium::msg::QueryMsg> = from_json(msg);
                assert_eq!(
                    query_msg,
                    Ok(okp4_cognitarium::msg::QueryMsg::Select {
                        query: SelectQuery {
                            prefixes: vec![],
                            limit: Some(1u32),
                            select: vec![SelectItem::Variable("p".to_string())],
                            r#where: vec![WhereCondition::Simple(
                                SimpleWhereCondition::TriplePattern(TriplePattern {
                                    subject: VarOrNode::Node(Node::NamedNode(IRI::Full(
                                        "http://example.edu/credentials/3732".to_string(),
                                    ))),
                                    predicate: VarOrNode::Variable("p".to_string()),
                                    object: VarOrNodeOrLiteral::Variable("o".to_string()),
                                })
                            )],
                        }
                    })
                );

                let select_resp = SelectResponse {
                    results: Results { bindings: vec![] },
                    head: Head { vars: vec![] },
                };
                SystemResult::Ok(ContractResult::Ok(to_json_binary(&select_resp).unwrap()))
            }
            _ => SystemResult::Err(SystemError::Unknown {}),
        });

        DATAVERSE
            .save(
                deps.as_mut().storage,
                &Dataverse {
                    name: "my-dataverse".to_string(),
                    triplestore_address: Addr::unchecked("my-dataverse-addr"),
                },
            )
            .unwrap();

        let resp = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("okp41072nc6egexqr2v6vpp7yxwm68plvqnkf6xsytf", &[]),
            ExecuteMsg::SubmitClaims {
                metadata: Binary(read_test_data("vc-eddsa-2020-ok.nq")),
                format: Some(RdfFormat::NQuads),
            },
        );

        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.messages.len(), 1);
        assert_eq!(
            resp.attributes,
            vec![
                Attribute::new("action", "submit_claims"),
                Attribute::new("credential", "http://example.edu/credentials/3732"),
                Attribute::new(
                    "subject",
                    "did:key:zDnaeUm3QkcyZWZTPttxB711jgqRDhkwvhF485SFw1bDZ9AQw"
                ),
                Attribute::new(
                    "type",
                    "https://example.org/examples#UniversityDegreeCredential"
                ),
            ]
        );

        let expected_data = "<http://example.edu/credentials/3732> <dataverse:credential#submitterAddress> <okp41072nc6egexqr2v6vpp7yxwm68plvqnkf6xsytf> .
<http://example.edu/credentials/3732> <dataverse:credential#issuer> <did:key:z6MkpwdnLPAm4apwcrRYQ6fZ3rAcqjLZR4AMk14vimfnozqY> .
<http://example.edu/credentials/3732> <dataverse:credential#type> <https://example.org/examples#UniversityDegreeCredential> .
<http://example.edu/credentials/3732> <dataverse:credential#validFrom> \"2024-02-16T00:00:00Z\"^^<http://www.w3.org/2001/XMLSchema#dateTime> .
<http://example.edu/credentials/3732> <dataverse:credential#subject> <did:key:zDnaeUm3QkcyZWZTPttxB711jgqRDhkwvhF485SFw1bDZ9AQw> .
_:c0 <https://example.org/examples#degree> _:b2 .
_:b2 <http://schema.org/name> \"Bachelor of Science and Arts\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML> .
_:b2 <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://example.org/examples#BachelorDegree> .
<http://example.edu/credentials/3732> <dataverse:credential#claim> _:c0 .
<http://example.edu/credentials/3732> <dataverse:credential#validUntil> \"2026-02-16T00:00:00Z\"^^<http://www.w3.org/2001/XMLSchema#dateTime> .\n";

        match resp.messages[0].msg.clone() {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            }) if contract_addr == "my-dataverse-addr".to_string() && funds == vec![] => {
                let exec_msg: StdResult<okp4_cognitarium::msg::ExecuteMsg> = from_json(msg);
                assert!(exec_msg.is_ok());
                match exec_msg.unwrap() {
                    okp4_cognitarium::msg::ExecuteMsg::InsertData { format, data } => {
                        assert_eq!(format, Some(DataFormat::NTriples));
                        assert_eq!(String::from_utf8(data.0).unwrap(), expected_data);
                    }
                    _ => assert!(false),
                }
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn submit_nonrdf_claims() {
        let resp = execute(
            mock_dependencies().as_mut(),
            mock_env(),
            mock_info("okp41072nc6egexqr2v6vpp7yxwm68plvqnkf6xsytf", &[]),
            ExecuteMsg::SubmitClaims {
                metadata: Binary("notrdf".as_bytes().to_vec()),
                format: Some(RdfFormat::NQuads),
            },
        );

        assert!(resp.is_err());
        assert!(matches!(resp.err().unwrap(), ContractError::ParseRDF(_)))
    }

    #[test]
    fn submit_invalid_claims() {
        let resp = execute(
            mock_dependencies().as_mut(),
            mock_env(),
            mock_info("okp41072nc6egexqr2v6vpp7yxwm68plvqnkf6xsytf", &[]),
            ExecuteMsg::SubmitClaims {
                metadata: Binary(vec![]),
                format: Some(RdfFormat::NQuads),
            },
        );

        assert!(resp.is_err());
        assert!(matches!(
            resp.err().unwrap(),
            ContractError::InvalidCredential(_)
        ))
    }

    #[test]
    fn submit_unverified_claims() {
        let resp = execute(
            mock_dependencies().as_mut(),
            mock_env(),
            mock_info("okp41072nc6egexqr2v6vpp7yxwm68plvqnkf6xsytf", &[]),
            ExecuteMsg::SubmitClaims {
                metadata: Binary(read_test_data("vc-eddsa-2020-ok-unsecured.nq")),
                format: Some(RdfFormat::NQuads),
            },
        );

        assert!(resp.is_err());
        assert!(matches!(
            resp.err().unwrap(),
            ContractError::CredentialVerification(_)
        ))
    }

    #[test]
    fn submit_unsupported_claims() {
        let resp = execute(
            mock_dependencies().as_mut(),
            mock_env(),
            mock_info("okp41072nc6egexqr2v6vpp7yxwm68plvqnkf6xsytf", &[]),
            ExecuteMsg::SubmitClaims {
                metadata: Binary(read_test_data("vc-unsupported-1.nq")),
                format: Some(RdfFormat::NQuads),
            },
        );

        assert!(resp.is_err());
        assert!(matches!(
            resp.err().unwrap(),
            ContractError::UnsupportedCredential(_)
        ))
    }

    #[test]
    fn submit_existing_claims() {
        let mut deps = mock_dependencies();
        deps.querier.update_wasm(|query| match query {
            WasmQuery::Smart { .. } => {
                let select_resp = SelectResponse {
                    results: Results {
                        bindings: vec![BTreeMap::from([(
                            "p".to_string(),
                            Value::BlankNode {
                                value: "".to_string(),
                            },
                        )])],
                    },
                    head: Head { vars: vec![] },
                };
                SystemResult::Ok(ContractResult::Ok(to_json_binary(&select_resp).unwrap()))
            }
            _ => SystemResult::Err(SystemError::Unknown {}),
        });

        DATAVERSE
            .save(
                deps.as_mut().storage,
                &Dataverse {
                    name: "my-dataverse".to_string(),
                    triplestore_address: Addr::unchecked("my-dataverse-addr"),
                },
            )
            .unwrap();

        let resp = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("okp41072nc6egexqr2v6vpp7yxwm68plvqnkf6xsytf", &[]),
            ExecuteMsg::SubmitClaims {
                metadata: Binary(read_test_data("vc-eddsa-2020-ok.nq")),
                format: Some(RdfFormat::NQuads),
            },
        );

        assert!(resp.is_err());
        assert!(
            matches!(resp.err().unwrap(), ContractError::CredentialAlreadyExists(id) if id == "http://example.edu/credentials/3732")
        );
    }
}
