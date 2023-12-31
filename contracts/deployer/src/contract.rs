#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    WasmMsg,
};
use cw0::parse_reply_instantiate_data;
use cw2::set_contract_version;
use cw_utils::{Duration, Threshold};

use crate::error::ContractError;
use crate::msg::{
    Cw3InstantiateMsg, Cw4InstantiateMsg, DeployMsg, ExecuteMsg, GetListOfWalletResponse,
    InstantiateMsg, QueryMsg,
};
use crate::state::{DEPLOY_DATA, GROUP_ADDR, USER_WALLETS};

/// version info for migration info
const CONTRACT_NAME: &str = "crates.io:deployer";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Handling contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

/// Handling contract execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deployer(deploy_msg) => execute_deployer(_deps, _env, _info, deploy_msg),
    }
}

pub fn execute_deployer(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _deploy_msg: DeployMsg,
) -> Result<Response, ContractError> {
    DEPLOY_DATA.save(_deps.storage, &_deploy_msg)?;

    let instantiate_cw4_group = WasmMsg::Instantiate {
        admin: None,
        code_id: 3035,
        msg: to_binary(&Cw4InstantiateMsg {
            admin: Some(_env.contract.address.to_string()),
            members: _deploy_msg.members,
        })?,
        funds: vec![],
        label: "cw4_group_instantiate".to_string(),
    };

    const INSTANTIATE_CW4_GROUP_MSG: u64 = 1u64;

    let submessage: SubMsg<Empty> =
        SubMsg::reply_on_success(instantiate_cw4_group, INSTANTIATE_CW4_GROUP_MSG);

    Ok(Response::new()
        .add_submessage(submessage)
        .add_attribute("method", "execute_deployer"))
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetListOfWallet { user_address } => {
            to_binary(&get_list_wallet(_deps, user_address))
        }
    }
}

pub fn get_list_wallet(
    _deps: Deps,
    user_address: String,
) -> Result<GetListOfWalletResponse, ContractError> {
    let wallet_array = USER_WALLETS.load(_deps.storage, user_address);
    match wallet_array {
        Ok(wallets) => Ok(GetListOfWalletResponse { wallets }),
        Err(_) => {
            return Err(ContractError::CustomError {
                val: "Unable to fetch user wallets".to_string(),
            })
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    const INSTANTIATE_CW4_GROUP_MSG: u64 = 1u64;
    const INSTANTIATE_CW3_FLEX_MULTISIG: u64 = 2u64;

    match _msg.id {
        INSTANTIATE_CW4_GROUP_MSG => reply::handle_instantiate_cw4_reply(_deps, _msg),
        INSTANTIATE_CW3_FLEX_MULTISIG => reply::handle_instantiate_cw3_flex_multisig(_deps, _msg),
        _id => {
            return Err(ContractError::CustomError {
                val: "ID Error!".to_string(),
            });
        }
    }
}

pub mod reply {
    use super::*;

    pub fn handle_instantiate_cw4_reply(
        _deps: DepsMut,
        _msg: Reply,
    ) -> Result<Response, ContractError> {
        let res = parse_reply_instantiate_data(_msg);

        match res {
            Ok(data) => {
                GROUP_ADDR.save(_deps.storage, &data.contract_address)?;

                let deploy_data = DEPLOY_DATA.load(_deps.storage);

                match deploy_data {
                    Ok(deploy) => {
                        let instantiate_cw3_flex_multisig = WasmMsg::Instantiate {
                            admin: None,
                            code_id: 3036,
                            msg: to_binary(&Cw3InstantiateMsg {
                                group_addr: data.contract_address,
                                threshold: Threshold::AbsoluteCount {
                                    weight: deploy.threshold_weight,
                                },
                                max_voting_period: Duration::Time(deploy.max_voting_period),
                                executor: None,
                                proposal_deposit: None,
                            })?,
                            funds: vec![],
                            label: "cw3_multi_sig".to_string(),
                        };

                        const INSTANTIATE_CW3_FLEX_MULTISIG: u64 = 2u64;

                        let submessage: SubMsg<Empty> = SubMsg::reply_on_success(
                            instantiate_cw3_flex_multisig,
                            INSTANTIATE_CW3_FLEX_MULTISIG,
                        );

                        let response = Response::new()
                            .add_submessage(submessage)
                            .add_attribute("method", "handle_instantiate_cw4_reply");
                        Ok(response)
                    }
                    Err(_) => {
                        return Err(ContractError::CustomError {
                            val: "Deploy Data Error!".to_string(),
                        })
                    }
                }
            }
            Err(_) => {
                return Err(ContractError::CustomError {
                    val: "Data Error!".to_string(),
                })
            }
        }
    }

    pub fn handle_instantiate_cw3_flex_multisig(
        _deps: DepsMut,
        _msg: Reply,
    ) -> Result<Response, ContractError> {
        let res = parse_reply_instantiate_data(_msg);

        match res {
            Ok(contract) => {
                let group_contract = GROUP_ADDR.load(_deps.storage);

                match group_contract {
                    Ok(data) => {
                        let deploy_data = DEPLOY_DATA.load(_deps.storage);

                        match deploy_data {
                            Ok(dm_data) => {
                                for address in dm_data.members.iter() {
                                    let wallet_save = USER_WALLETS.update(
                                        _deps.storage,
                                        address.addr.clone(),
                                        |mul_wallet| -> StdResult<Vec<String>> {
                                            match mul_wallet {
                                                Some(mut wallets) => {
                                                    wallets.push(contract.contract_address.clone());
                                                    Ok(wallets)
                                                }
                                                None => {
                                                    let mut wallet_array: Vec<String> = Vec::new();
                                                    wallet_array
                                                        .push(contract.contract_address.clone());
                                                    Ok(wallet_array)
                                                }
                                            }
                                        },
                                    );

                                    match wallet_save {
                                        Ok(_) => {}
                                        Err(_) => {
                                            return Err(ContractError::CustomError {
                                                val: "Unable to update user wallets!".to_string(),
                                            });
                                        }
                                    }
                                }

                                let execute_cw4_addhook = WasmMsg::Execute {
                                    contract_addr: data.clone(),
                                    msg: to_binary(&cw4::Cw4ExecuteMsg::AddHook {
                                        addr: contract.contract_address.clone(),
                                    })?,
                                    funds: vec![],
                                };

                                let execute_cw4_update = WasmMsg::Execute {
                                    contract_addr: data,
                                    msg: to_binary(&cw4::Cw4ExecuteMsg::UpdateAdmin {
                                        admin: Some(contract.contract_address),
                                    })?,
                                    funds: vec![],
                                };

                                Ok(Response::new()
                                    .add_message(execute_cw4_addhook)
                                    .add_message(execute_cw4_update)
                                    .add_attribute(
                                        "method",
                                        "handle_instantiate_cw3_flex_multisig",
                                    ))
                            }
                            Err(_) => {
                                return Err(ContractError::CustomError {
                                    val: "Can't Find Deploy Msg!".to_string(),
                                })
                            }
                        }
                    }
                    Err(_) => {
                        return Err(ContractError::CustomError {
                            val: "cw4_group_addr not found!".to_string(),
                        })
                    }
                }
            }
            Err(_) => {
                return Err(ContractError::CustomError {
                    val: "Contract Error!".to_string(),
                })
            }
        }
    }
}
