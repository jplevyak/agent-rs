//! In this file, please mark all tests that require a running ic-ref as ignored.
//!
//! These tests are a Rust-like version using the Agent to cover the same tests
//! as the IC Ref repo itself.
//!
//! The tests can be found in the Spec.hs file in the IC Ref repo.
//!
//! Try to keep these tests as close to 1-to-1 to the IC Ref test use cases. For
//! every spec in the IC Ref tests, there should be a matching spec here. Some
//! tests (like invalid CBOR or special Headers) might not be translatable, in
//! which case they should still be added here but do nothing (just keep the
//! use case being tested).
use ref_tests::universal_canister;
use ref_tests::with_agent;

const EXPECTED_IC_API_VERSION: &str = "0.16.0";

#[ignore]
#[test]
fn status_endpoint() {
    with_agent(|agent| async move {
        agent.status().await?;
        Ok(())
    })
}

#[ignore]
#[test]
fn spec_compliance_claimed() {
    with_agent(|agent| async move {
        let status = agent.status().await?;

        assert_eq!(status.ic_api_version, EXPECTED_IC_API_VERSION);

        Ok(())
    });
}

mod management_canister {
    use ic_agent::export::Principal;
    use ic_agent::AgentError;
    use ic_utils::call::AsyncCall;
    use ic_utils::interfaces::management_canister::{
        CanisterStatus, InstallMode, StatusCallResult,
    };
    use ic_utils::interfaces::wallet::CreateResult;
    use ic_utils::interfaces::{ManagementCanister, Wallet};
    use ic_utils::{Argument, Canister};
    use openssl::sha::Sha256;
    use ref_tests::{
        create_agent, create_basic_identity, create_waiter, with_agent, with_wallet_canister,
    };

    mod create_canister {
        use super::{create_waiter, with_agent};
        use ic_agent::export::Principal;
        use ic_agent::AgentError;
        use ic_utils::call::AsyncCall;
        use ic_utils::interfaces::ManagementCanister;
        use std::str::FromStr;

        #[ignore]
        #[test]
        fn no_id_given() {
            with_agent(|agent| async move {
                let ic00 = ManagementCanister::create(&agent);

                let _ = ic00
                    .provisional_create_canister_with_cycles(None)
                    .call_and_wait(create_waiter())
                    .await?;

                Ok(())
            })
        }

        #[ignore]
        #[test]
        fn create_canister_necessary() {
            with_agent(|agent| async move {
                let ic00 = ManagementCanister::create(&agent);
                let canister_wasm = b"\0asm\x01\0\0\0".to_vec();

                let result = ic00
                    .install_code(
                        &Principal::from_str("75hes-oqbaa-aaaaa-aaaaa-aaaaa-aaaaa-aaaaa-q")
                            .unwrap(),
                        &canister_wasm,
                    )
                    .call_and_wait(create_waiter())
                    .await;

                let payload_content =
                    "canister does not exist: 75hes-oqbaa-aaaaa-aaaaa-aaaaa-aaaaa-aaaaa-q"
                        .to_string();

                assert!(matches!(result,
                    Err(AgentError::HttpError(payload))
                        if String::from_utf8(payload.content.clone()).expect("Expected utf8") == payload_content));
                Ok(())
            })
        }
    }

    #[ignore]
    #[test]
    fn management() {
        with_agent(|agent| async move {
            let ic00 = ManagementCanister::create(&agent);

            let (canister_id,) = ic00
                .provisional_create_canister_with_cycles(None)
                .call_and_wait(create_waiter())
                .await?;
            let canister_wasm = b"\0asm\x01\0\0\0".to_vec();

            // Install once.
            ic00.install_code(&canister_id, &canister_wasm)
                .with_mode(InstallMode::Install)
                .call_and_wait(create_waiter())
                .await?;

            // Re-install should fail.
            let result = ic00
                .install_code(&canister_id, &canister_wasm)
                .with_mode(InstallMode::Install)
                .call_and_wait(create_waiter())
                .await;

            assert!(matches!(result, Err(AgentError::ReplicaError { .. })));

            // Reinstall should succeed.
            ic00.install_code(&canister_id, &canister_wasm)
                .with_mode(InstallMode::Reinstall)
                .call_and_wait(create_waiter())
                .await?;

            // Each agent has their own identity.
            let other_agent_identity = create_basic_identity().await?;
            let other_agent_principal = other_agent_identity.sender()?;
            let other_agent = create_agent(other_agent_identity).await?;
            other_agent.fetch_root_key().await?;
            let other_ic00 = ManagementCanister::create(&other_agent);

            // Reinstall with another agent should fail.
            let result = other_ic00
                .install_code(&canister_id, &canister_wasm)
                .with_mode(InstallMode::Reinstall)
                .call_and_wait(create_waiter())
                .await;
            assert!(matches!(result, Err(AgentError::HttpError(..))));

            // Upgrade should succeed.
            ic00.install_code(&canister_id, &canister_wasm)
                .with_mode(InstallMode::Upgrade)
                .call_and_wait(create_waiter())
                .await?;

            // Upgrade with another agent should fail.
            let result = other_ic00
                .install_code(&canister_id, &canister_wasm)
                .with_mode(InstallMode::Upgrade)
                .call_and_wait(create_waiter())
                .await;
            assert!(matches!(result, Err(AgentError::HttpError(..))));

            // Change controller.
            ic00.set_controller(&canister_id, &other_agent_principal)
                .call_and_wait(create_waiter())
                .await?;

            // Change controller with wrong controller should fail
            let result = ic00
                .set_controller(&canister_id, &other_agent_principal)
                .call_and_wait(create_waiter())
                .await;
            assert!(matches!(result, Err(AgentError::HttpError(payload))
                if String::from_utf8(payload.content.clone()).expect("Expected utf8")
                    == *"Wrong sender"));

            // Reinstall as new controller
            other_ic00
                .install_code(&canister_id, &canister_wasm)
                .with_mode(InstallMode::Reinstall)
                .call_and_wait(create_waiter())
                .await?;

            // Reinstall on empty should succeed.
            let (canister_id_2,) = ic00
                .provisional_create_canister_with_cycles(None)
                .call_and_wait(create_waiter())
                .await?;

            // Reinstall over empty canister
            ic00.install_code(&canister_id_2, &canister_wasm)
                .with_mode(InstallMode::Reinstall)
                .call_and_wait(create_waiter())
                .await?;

            // Create an empty canister
            let (canister_id_3,) = other_ic00
                .provisional_create_canister_with_cycles(None)
                .call_and_wait(create_waiter())
                .await?;

            // Check status for empty canister
            let result = other_ic00
                .canister_status(&canister_id_3)
                .call_and_wait(create_waiter())
                .await?;
            assert_eq!(result.0.status, CanisterStatus::Running);
            assert_eq!(result.0.controller, other_agent_principal);
            assert_eq!(result.0.module_hash, None);

            // Install wasm.
            other_ic00
                .install_code(&canister_id_3, &canister_wasm)
                .with_mode(InstallMode::Install)
                .call_and_wait(create_waiter())
                .await?;

            // Check status after installing wasm and validate module_hash
            let result = other_ic00
                .canister_status(&canister_id_3)
                .call_and_wait(create_waiter())
                .await?;
            let mut hasher = Sha256::new();
            hasher.update(&canister_wasm);
            let sha256_digest = hasher.finish();
            assert_eq!(result.0.module_hash, Some(sha256_digest.into()));

            Ok(())
        })
    }

    #[ignore]
    #[test]
    fn canister_lifecycle_and_delete() {
        with_agent(|agent| async move {
            let ic00 = ManagementCanister::create(&agent);
            let (canister_id,) = ic00
                .provisional_create_canister_with_cycles(None)
                .call_and_wait(create_waiter())
                .await?;
            let canister_wasm = b"\0asm\x01\0\0\0".to_vec();

            // Install once.
            ic00.install_code(&canister_id, &canister_wasm)
                .with_mode(InstallMode::Install)
                .call_and_wait(create_waiter())
                .await?;

            // A newly installed canister should be running
            let result = ic00
                .canister_status(&canister_id)
                .call_and_wait(create_waiter())
                .await;
            assert_eq!(result?.0.status, CanisterStatus::Running);

            // Stop should succeed.
            ic00.stop_canister(&canister_id)
                .call_and_wait(create_waiter())
                .await?;

            // Canister should be stopped
            let result = ic00
                .canister_status(&canister_id)
                .call_and_wait(create_waiter())
                .await;
            assert_eq!(result?.0.status, CanisterStatus::Stopped);

            // Another stop is a noop
            ic00.stop_canister(&canister_id)
                .call_and_wait(create_waiter())
                .await?;

            // Can't call update on a stopped canister
            let result = agent
                .update(&canister_id, "update")
                .call_and_wait(create_waiter())
                .await;
            assert!(matches!(result, Err(AgentError::HttpError(payload))
                if String::from_utf8(payload.content.clone()).expect("Expected utf8") == *"canister is stopped"));

            // Can't call query on a stopped canister
            let result = agent
                .query(&canister_id, "query")
                .with_arg(&[])
                .call()
                .await;
            assert!(matches!(result, Err(AgentError::ReplicaError {
                    reject_code: 5,
                    reject_message,
                }) if reject_message == "canister is stopped"));

            // Start should succeed.
            ic00.start_canister(&canister_id)
                .call_and_wait(create_waiter())
                .await?;

            // Canister should be running
            let result = ic00
                .canister_status(&canister_id)
                .call_and_wait(create_waiter())
                .await;
            assert_eq!(result?.0.status, CanisterStatus::Running);

            // Can call update
            let result = agent
                .update(&canister_id, "update")
                .call_and_wait(create_waiter())
                .await;
            assert!(matches!(result, Err(AgentError::ReplicaError {
                    reject_code: 3,
                    reject_message,
                }) if reject_message == "method does not exist: update"));

            // Can call query
            let result = agent
                .query(&canister_id, "query")
                .with_arg(&[])
                .call()
                .await;
            assert!(matches!(result, Err(AgentError::ReplicaError {
                    reject_code: 3,
                    reject_message,
                }) if reject_message == "query method does not exist"));

            // Another start is a noop
            ic00.start_canister(&canister_id)
                .call_and_wait(create_waiter())
                .await?;

            // Delete a running canister should fail.
            let result = ic00
                .delete_canister(&canister_id)
                .call_and_wait(create_waiter())
                .await;
            assert!(matches!(result, Err(AgentError::ReplicaError { .. })));

            // Stop should succeed.
            ic00.stop_canister(&canister_id)
                .call_and_wait(create_waiter())
                .await?;

            // Delete a stopped canister succeeds.
            ic00.delete_canister(&canister_id)
                .call_and_wait(create_waiter())
                .await?;

            // Cannot call update
            let result = agent
                .update(&canister_id, "update")
                .call_and_wait(create_waiter())
                .await;
            assert!(matches!(result, Err(AgentError::HttpError(payload))
                if String::from_utf8(payload.content.clone()).expect("Expected utf8")
                    == format!("canister no longer exists: {}", canister_id.to_text())));

            // Cannot call query
            let result = agent
                .query(&canister_id, "query")
                .with_arg(&[])
                .call()
                .await;
            assert!(matches!(result, Err(AgentError::ReplicaError {
                    reject_code: 3,
                    reject_message,
                }) if reject_message
                    == format!("canister no longer exists: {}", canister_id.to_text())));

            // Cannot query canister status
            let result = ic00
                .canister_status(&canister_id)
                .call_and_wait(create_waiter())
                .await;
            assert!(match result {
                Err(AgentError::HttpError(payload))
                    if String::from_utf8(payload.content.clone()).expect("Expected utf8")
                        == format!("canister no longer exists: {}", canister_id.to_text()) =>
                    true,
                Ok((_status_call_result,)) => false,
                _ => false,
            });

            // Delete a running canister should fail.
            let result = ic00
                .delete_canister(&canister_id)
                .call_and_wait(create_waiter())
                .await;
            assert!(matches!(result, Err(AgentError::HttpError(payload))
                if String::from_utf8(payload.content.clone()).expect("Expected utf8")
                    == format!("canister no longer exists: {}", canister_id.to_text())));
            Ok(())
        })
    }

    #[ignore]
    #[test]
    fn canister_lifecycle_as_wrong_controller() {
        with_agent(|agent| async move {
            let ic00 = ManagementCanister::create(&agent);
            let (canister_id,) = ic00
                .provisional_create_canister_with_cycles(None)
                .call_and_wait(create_waiter())
                .await?;
            let canister_wasm = b"\0asm\x01\0\0\0".to_vec();

            // Install once.
            ic00.install_code(&canister_id, &canister_wasm)
                .with_mode(InstallMode::Install)
                .call_and_wait(create_waiter())
                .await?;

            // Create another agent with different identity.
            let other_agent_identity = create_basic_identity().await?;
            let other_agent = create_agent(other_agent_identity).await?;
            other_agent.fetch_root_key().await?;
            let other_ic00 = ManagementCanister::create(&other_agent);

            // Start as a wrong controller should fail.
            let result = other_ic00
                .start_canister(&canister_id)
                .call_and_wait(create_waiter())
                .await;
            assert!(matches!(result, Err(AgentError::HttpError(payload))
                if String::from_utf8(payload.content.clone()).expect("Expected utf8")
                    == *"Wrong sender"));

            // Stop as a wrong controller should fail.
            let result = other_ic00
                .stop_canister(&canister_id)
                .call_and_wait(create_waiter())
                .await;
            assert!(matches!(result, Err(AgentError::HttpError(payload))
                if String::from_utf8(payload.content.clone()).expect("Expected utf8")
                    == *"Wrong sender"));

            // Get canister status as a wrong controller should fail.
            let result = other_ic00
                .canister_status(&canister_id)
                .call_and_wait(create_waiter())
                .await;
            assert!(matches!(result, Err(AgentError::HttpError(payload))
                if String::from_utf8(payload.content.clone()).expect("Expected utf8")
                    == *"Wrong sender"));

            // Delete as a wrong controller should fail.
            let result = other_ic00
                .delete_canister(&canister_id)
                .call_and_wait(create_waiter())
                .await;
            assert!(matches!(result, Err(AgentError::HttpError(payload))
                if String::from_utf8(payload.content.clone()).expect("Expected utf8")
                    == *"Wrong sender"));

            Ok(())
        })
    }

    #[ignore]
    #[test]
    fn provisional_create_canister_with_cycles() {
        with_wallet_canister(None, |agent, wallet_id| async move {
            let max_canister_balance: u64 = 1152921504606846976;

            // empty cycle balance on create
            let wallet = Wallet::create(&agent, wallet_id);
            let ic00 = Canister::builder()
                .with_agent(&agent)
                .with_canister_id(Principal::management_canister())
                .build()?;
            let (create_result,): (CreateResult,) = wallet
                .call(&ic00, "create_canister", Argument::default(), 0)
                .call_and_wait(create_waiter())
                .await?;
            let canister_id = create_result.canister_id;

            #[derive(candid::CandidType)]
            struct In {
                canister_id: Principal,
            }
            let status_args = In { canister_id };
            let mut args = Argument::default();
            args.push_idl_arg(status_args);

            let (result,): (StatusCallResult,) = wallet
                .call(&ic00, "canister_status", args, 0)
                .call_and_wait(create_waiter())
                .await?;

            assert_eq!(result.cycles, 0_u64);

            let ic00 = ManagementCanister::create(&agent);
            // cycle balance is max_canister_balance when creating with
            // provisional_create_canister_with_cycles(None)
            let (canister_id_1,) = ic00
                .provisional_create_canister_with_cycles(None)
                .call_and_wait(create_waiter())
                .await?;
            let result = ic00
                .canister_status(&canister_id_1)
                .call_and_wait(create_waiter())
                .await?;
            assert_eq!(result.0.cycles, max_canister_balance);

            // cycle balance should be amount specified to
            // provisional_create_canister_with_cycles call
            let amount: u64 = 1 << 40; // 1099511627776
            let (canister_id_2,) = ic00
                .provisional_create_canister_with_cycles(Some(amount))
                .call_and_wait(create_waiter())
                .await?;
            let result = ic00
                .canister_status(&canister_id_2)
                .call_and_wait(create_waiter())
                .await?;
            assert_eq!(result.0.cycles, amount);

            Ok(())
        })
    }

    #[ignore]
    #[test]
    fn randomness() {
        with_wallet_canister(None, |agent, wallet_id| async move {
            let wallet = Wallet::create(&agent, wallet_id);
            let ic00 = Canister::builder()
                .with_agent(&agent)
                .with_canister_id(Principal::management_canister())
                .build()?;
            let (rand_1,): (Vec<u8>,) = wallet
                .call(&ic00, "raw_rand", Argument::default(), 0)
                .call_and_wait(create_waiter())
                .await?;
            let (rand_2,): (Vec<u8>,) = wallet
                .call(&ic00, "raw_rand", Argument::default(), 0)
                .call_and_wait(create_waiter())
                .await?;
            let (rand_3,): (Vec<u8>,) = wallet
                .call(&ic00, "raw_rand", Argument::default(), 0)
                .call_and_wait(create_waiter())
                .await?;

            assert_eq!(rand_1.len(), 32);
            assert_eq!(rand_2.len(), 32);
            assert_eq!(rand_3.len(), 32);

            assert_ne!(rand_1, rand_2);
            assert_ne!(rand_1, rand_3);
            assert_ne!(rand_2, rand_3);

            Ok(())
        })
    }
}

mod simple_calls {
    use crate::universal_canister::payload;
    use ic_agent::AgentError;
    use ref_tests::{create_waiter, with_universal_canister};

    #[ignore]
    #[test]
    fn call() {
        with_universal_canister(|agent, canister_id| async move {
            let arg = payload().reply_data(b"hello").build();
            let result = agent
                .update(&canister_id, "update")
                .with_arg(&arg)
                .call_and_wait(create_waiter())
                .await?;

            assert_eq!(result.as_slice(), b"hello");
            Ok(())
        })
    }

    #[ignore]
    #[test]
    fn query() {
        with_universal_canister(|agent, canister_id| async move {
            let arg = payload().reply_data(b"hello").build();
            let result = agent
                .query(&canister_id, "query")
                .with_arg(arg)
                .call()
                .await?;

            assert_eq!(result, b"hello");
            Ok(())
        })
    }

    #[ignore]
    #[test]
    fn non_existant_call() {
        with_universal_canister(|agent, canister_id| async move {
            let arg = payload().reply_data(b"hello").build();
            let result = agent
                .update(&canister_id, "non_existent_method")
                .with_arg(&arg)
                .call_and_wait(create_waiter())
                .await;

            assert!(matches!(
                result,
                Err(AgentError::ReplicaError { reject_code: 3, .. })
            ));
            Ok(())
        })
    }

    #[ignore]
    #[test]
    fn non_existant_query() {
        with_universal_canister(|agent, canister_id| async move {
            let arg = payload().reply_data(b"hello").build();
            let result = agent
                .query(&canister_id, "non_existent_method")
                .with_arg(&arg)
                .call()
                .await;

            assert!(matches!(
                result,
                Err(AgentError::ReplicaError { reject_code: 3, .. })
            ));
            Ok(())
        })
    }
}

mod extras {
    use ic_utils::call::AsyncCall;
    use ic_utils::interfaces::management_canister::ComputeAllocation;
    use ic_utils::interfaces::ManagementCanister;
    use ref_tests::{create_waiter, with_agent};

    #[ignore]
    #[test]
    fn memory_allocation() {
        with_agent(|agent| async move {
            let ic00 = ManagementCanister::create(&agent);
            let (canister_id,) = ic00
                .provisional_create_canister_with_cycles(None)
                .call_and_wait(create_waiter())
                .await?;
            let canister_wasm = b"\0asm\x01\0\0\0".to_vec();

            // Prevent installing with over 1 << 48. This does not contact the server.
            assert!(ic00
                .install_code(&canister_id, &canister_wasm)
                .with_memory_allocation(1u64 << 50)
                .call_and_wait(create_waiter())
                .await
                .is_err());

            ic00.install_code(&canister_id, &canister_wasm)
                .with_memory_allocation(10 * 1024 * 1024u64)
                .call_and_wait(create_waiter())
                .await?;

            Ok(())
        })
    }

    #[ignore]
    #[test]
    fn compute_allocation() {
        use std::convert::TryFrom;

        with_agent(|agent| async move {
            let ic00 = ManagementCanister::create(&agent);
            let (canister_id,) = ic00
                .provisional_create_canister_with_cycles(None)
                .call_and_wait(create_waiter())
                .await?;
            let canister_wasm = b"\0asm\x01\0\0\0".to_vec();

            let ca = ComputeAllocation::try_from(10).unwrap();

            ic00.install_code(&canister_id, &canister_wasm)
                .with_compute_allocation(ca)
                .call_and_wait(create_waiter())
                .await?;

            Ok(())
        })
    }
}
