//! Implementations of [`Testing`](spec::Group::Testing) cheatcodes.

use crate::{Cheatcode, Cheatcodes, CheatsCtxt, DatabaseExt, Error, Result, Vm::*};
use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use foundry_evm_core::constants::{MAGIC_ASSUME, MAGIC_SKIP};
use foundry_zksync_compiler::DualCompiledContract;

pub(crate) mod assert;
pub(crate) mod expect;

impl Cheatcode for zkVmCall {
    fn apply_stateful<DB: DatabaseExt>(&self, ccx: &mut CheatsCtxt<DB>) -> Result {
        let Self { enable } = *self;

        if enable {
            ccx.state.select_zk_vm(ccx.ecx, None);
        } else {
            ccx.state.select_evm(ccx.ecx);
        }

        Ok(Default::default())
    }
}

impl Cheatcode for zkRegisterContractCall {
    fn apply_stateful<DB: DatabaseExt>(&self, ccx: &mut CheatsCtxt<DB>) -> Result {
        let Self {
            name,
            evmBytecodeHash,
            evmDeployedBytecode,
            evmBytecode,
            zkBytecodeHash,
            zkDeployedBytecode,
        } = self;

        let new_contract = DualCompiledContract {
            name: name.clone(),
            zk_bytecode_hash: zkBytecodeHash.0.into(),
            zk_deployed_bytecode: zkDeployedBytecode.to_vec(),
            //TODO: add argument to cheatcode
            zk_factory_deps: vec![],
            evm_bytecode_hash: *evmBytecodeHash,
            evm_deployed_bytecode: evmDeployedBytecode.to_vec(),
            evm_bytecode: evmBytecode.to_vec(),
        };

        if let Some(existing) = ccx.state.dual_compiled_contracts.iter().find(|contract| {
            contract.evm_bytecode_hash == new_contract.evm_bytecode_hash &&
                contract.zk_bytecode_hash == new_contract.zk_bytecode_hash
        }) {
            warn!(name = existing.name, "contract already exists with the given bytecode hashes");
            return Ok(Default::default())
        }

        ccx.state.dual_compiled_contracts.push(new_contract);

        Ok(Default::default())
    }
}

impl Cheatcode for assumeCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        let Self { condition } = self;
        if *condition {
            Ok(Default::default())
        } else {
            Err(Error::from(MAGIC_ASSUME))
        }
    }
}

impl Cheatcode for breakpoint_0Call {
    fn apply_stateful<DB: DatabaseExt>(&self, ccx: &mut CheatsCtxt<DB>) -> Result {
        let Self { char } = self;
        breakpoint(ccx.state, &ccx.caller, char, true)
    }
}

impl Cheatcode for breakpoint_1Call {
    fn apply_stateful<DB: DatabaseExt>(&self, ccx: &mut CheatsCtxt<DB>) -> Result {
        let Self { char, value } = self;
        breakpoint(ccx.state, &ccx.caller, char, *value)
    }
}

impl Cheatcode for rpcUrlCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self { rpcAlias } = self;
        state.config.rpc_url(rpcAlias).map(|url| url.abi_encode())
    }
}

impl Cheatcode for rpcUrlsCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self {} = self;
        state.config.rpc_urls().map(|urls| urls.abi_encode())
    }
}

impl Cheatcode for rpcUrlStructsCall {
    fn apply(&self, state: &mut Cheatcodes) -> Result {
        let Self {} = self;
        state.config.rpc_urls().map(|urls| urls.abi_encode())
    }
}

impl Cheatcode for sleepCall {
    fn apply(&self, _state: &mut Cheatcodes) -> Result {
        let Self { duration } = self;
        let sleep_duration = std::time::Duration::from_millis(duration.saturating_to());
        std::thread::sleep(sleep_duration);
        Ok(Default::default())
    }
}

impl Cheatcode for skipCall {
    fn apply_stateful<DB: DatabaseExt>(&self, ccx: &mut CheatsCtxt<DB>) -> Result {
        let Self { skipTest } = *self;
        if skipTest {
            // Skip should not work if called deeper than at test level.
            // Since we're not returning the magic skip bytes, this will cause a test failure.
            ensure!(ccx.ecx.journaled_state.depth() <= 1, "`skip` can only be used at test level");
            Err(MAGIC_SKIP.into())
        } else {
            Ok(Default::default())
        }
    }
}

/// Adds or removes the given breakpoint to the state.
fn breakpoint(state: &mut Cheatcodes, caller: &Address, s: &str, add: bool) -> Result {
    let mut chars = s.chars();
    let (Some(point), None) = (chars.next(), chars.next()) else {
        bail!("breakpoints must be exactly one character");
    };
    ensure!(point.is_alphabetic(), "only alphabetic characters are accepted as breakpoints");

    if add {
        state.breakpoints.insert(point, (*caller, state.pc));
    } else {
        state.breakpoints.remove(&point);
    }

    Ok(Default::default())
}
