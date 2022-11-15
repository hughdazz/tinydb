#![cfg_attr(debug_assertions, allow(dead_code))]

use std::{
    collections::HashMap,
    io::{BufRead, Cursor, Write},
};

use byteorder::{BigEndian, ByteOrder, WriteBytesExt};

use crate::sql::RowDescriptor;

pub const AUTH_TYPE_OK: u32 = 0;
pub const PROTOCOL_VERSION_NUMBER: u32 = 196608; // 3.0
pub const SSL_REQUEST_NUMBER: u32 = 80877103;
pub const CANCEL_REQUEST_CODE: u32 = 80877102;
pub const GSS_ENC_REQ_NUMBER: u32 = 80877104;

pub const PARSE_COMPLETE_TAG: u8 = b'1';
pub const BIND_COMPLETE_TAG: u8 = b'2';
pub const CLOSE_COMPLETE_TAG: u8 = b'3';
pub const NOTIFICATION_RESPONSE_TAG: u8 = b'A';
pub const COPY_DONE_TAG: u8 = b'c';
pub const COMMAND_COMPLETE_TAG: u8 = b'C';
pub const COPY_DATA_TAG: u8 = b'd';
pub const DATA_ROW_TAG: u8 = b'D';
pub const ERROR_RESPONSE_TAG: u8 = b'E';
pub const COPY_IN_RESPONSE_TAG: u8 = b'G';
pub const COPY_OUT_RESPONSE_TAG: u8 = b'H';
pub const EMPTY_QUERY_RESPONSE_TAG: u8 = b'I';
pub const BACKEND_KEY_DATA_TAG: u8 = b'K';
pub const NO_DATA_TAG: u8 = b'n';
pub const NOTICE_RESPONSE_TAG: u8 = b'N';
pub const AUTHENTICATION_TAG: u8 = b'R';
pub const PORTAL_SUSPENDED_TAG: u8 = b's';
pub const PARAMETER_STATUS_TAG: u8 = b'S';
pub const PARAMETER_DESCRIPTION_TAG: u8 = b't';
pub const ROW_DESCRIPTION_TAG: u8 = b'T';
pub const READY_FOR_QUERY_TAG: u8 = b'Z';

#[derive(Debug)]
pub enum FrontendMessage {
    StartupMessage(StartupMessage),
    Query(Query),
}

#[derive(Debug)]
pub struct Query {
    pub query: String,
}

impl Query {
    pub fn decode<R>(decode_from: &mut R) -> anyhow::Result<FrontendMessage>
    where
        R: byteorder::ReadBytesExt,
    {
        let msg_len = decode_from.read_u32::<BigEndian>()?;

        // Exclude the msg_len when reading
        let mut msg_body = vec![0; (msg_len as usize) - 4];
        decode_from.read(&mut msg_body)?;

        // Exclude the \0 at the end when parsing.
        let _ = msg_body.pop();
        let query = String::from_utf8(msg_body)?;
        Ok(FrontendMessage::Query(Self { query }))
    }
}

pub struct ReadyForQuery;

impl ReadyForQuery {
    pub fn encode<W>(encode_to: &mut W) -> anyhow::Result<()>
    where
        W: Write,
    {
        // encode BackendKeyData message
        {
            encode_to.write_u8(BACKEND_KEY_DATA_TAG)?;
            // message lenght
            encode_to.write_u32::<BigEndian>(12)?;
            // process id
            encode_to.write_u32::<BigEndian>(42)?;
            // secret key
            encode_to.write_u32::<BigEndian>(12345)?;
        }

        // Encode ParameterStatus message
        {
            let key = "TimeZone";
            let value = "America/Sao_Paulo";

            let mut buf = Vec::new();
            buf.write(key.as_bytes())?;
            buf.write_u8(0)?;
            buf.write(value.as_bytes())?;
            buf.write_u8(0)?;

            encode_to.write_u8(PARAMETER_STATUS_TAG)?;
            encode_to.write_i32::<BigEndian>((buf.len() as i32) + 4)?;
            encode_to.write(&buf)?;
        }

        encode_to.write(&[READY_FOR_QUERY_TAG, 0, 0, 0, 5, EMPTY_QUERY_RESPONSE_TAG])?;
        Ok(())
    }
}

pub struct CommandComplete;

impl CommandComplete {
    pub fn encode<W>(encode_to: &mut W) -> anyhow::Result<()>
    where
        W: Write,
    {
        let tag = "SELECT 0".as_bytes();

        encode_to.write_u8(COMMAND_COMPLETE_TAG)?;
        encode_to.write_i32::<BigEndian>((tag.len() as i32) + 5)?;
        encode_to.write(&tag)?;
        encode_to.write_u8(0)?;
        Ok(())
    }
}

impl RowDescriptor {
    pub fn encode<W>(&self, encode_to: &mut W) -> anyhow::Result<()>
    where
        W: Write,
    {
        let mut field_values = Vec::new();

        field_values.write_u16::<BigEndian>(self.fields.len() as u16)?;
        for field in &self.fields {
            field_values.write(&field.name)?;
            field_values.write_u8(0)?;

            field_values.write_u32::<BigEndian>(field.table_oid)?;
            field_values.write_u16::<BigEndian>(field.table_attribute_number)?;
            field_values.write_u32::<BigEndian>(field.data_type_oid)?;
            field_values.write_i16::<BigEndian>(field.data_type_size)?;
            field_values.write_i32::<BigEndian>(field.type_modifier)?;
            field_values.write_i16::<BigEndian>(field.format)?;
        }

        encode_to.write_u8(ROW_DESCRIPTION_TAG)?;
        encode_to.write_i32::<BigEndian>((field_values.len() as i32) + 4)?;
        encode_to.write(&field_values)?;

        Ok(())
    }
}

pub struct AuthenticationOk;

impl AuthenticationOk {
    pub fn encode<W>(encode_to: &mut W) -> anyhow::Result<()>
    where
        W: byteorder::WriteBytesExt,
    {
        encode_to.write(&[AUTHENTICATION_TAG])?;
        encode_to.write_i32::<BigEndian>(8)?;
        encode_to.write_u32::<BigEndian>(AUTH_TYPE_OK)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct StartupMessage {
    pub protocol_version: u32,
    pub parameters: HashMap<String, String>,
}

impl StartupMessage {
    pub fn decode(src: &[u8]) -> anyhow::Result<FrontendMessage> {
        if src.len() < 4 {
            anyhow::bail!("startup message to short");
        }

        let protocol_version = BigEndian::read_u32(src);

        let mut parameters = HashMap::new();

        let mut cursor = Cursor::new(&src[4..]);
        loop {
            let mut buf = Vec::new();
            cursor.read_until(0, &mut buf)?;
            if buf.is_empty() {
                break;
            }

            let key = String::from_utf8(buf)?;

            let mut buf = Vec::new();
            cursor.read_until(0, &mut buf)?;
            let value = String::from_utf8(buf)?;

            parameters.insert(key, value);
        }

        Ok(FrontendMessage::StartupMessage(Self {
            protocol_version,
            parameters,
        }))
    }
}
