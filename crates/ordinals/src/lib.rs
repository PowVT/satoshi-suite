use std::{error::Error, io::Cursor, mem, str::FromStr};

use bitcoin::{
    constants::MAX_SCRIPT_ELEMENT_SIZE,
    opcodes,
    script::{Builder as ScriptBuilder, PushBytes, PushBytesBuf, ScriptBuf},
};

use ord::{Chain, Inscription};

use serde::{Deserialize, Serialize};

use serde_json::to_string;
use utils::push_bytes::bytes_to_push_bytes;

mod utils;

use crate::utils::constants;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct InscriptionData {
    pub body: Option<Vec<u8>>,
    pub content_encoding: Option<Vec<u8>>,
    pub content_type: Option<Vec<u8>>,
    pub delegate: Option<Vec<u8>>,
    pub duplicate_field: bool,
    pub incomplete_field: bool,
    pub metadata: Option<Vec<u8>>,
    pub metaprotocol: Option<Vec<u8>>,
    pub parents: Vec<Vec<u8>>,
    pub pointer: Option<Vec<u8>>,
    pub rune: Option<Vec<u8>>,
    pub unrecognized_even_field: bool,
}

impl InscriptionData {
    pub fn new(chain: Chain, path: &str) -> Result<Self, Box<dyn Error>> {
        let ord_inscription = Inscription::new(
            chain,
            false,
            None,
            None,
            None,
            Vec::new(),
            Some(std::path::PathBuf::from(path)),
            None,
            None,
        )?;

        // Convert ord::Inscription to InscriptionData
        Ok(Self {
            body: ord_inscription.body,
            content_encoding: ord_inscription.content_encoding,
            content_type: ord_inscription.content_type,
            delegate: ord_inscription.delegate,
            duplicate_field: ord_inscription.duplicate_field,
            incomplete_field: ord_inscription.incomplete_field,
            metadata: ord_inscription.metadata,
            metaprotocol: ord_inscription.metaprotocol,
            parents: ord_inscription.parents,
            pointer: ord_inscription.pointer,
            rune: ord_inscription.rune,
            unrecognized_even_field: ord_inscription.unrecognized_even_field,
        })
    }

    pub fn append_reveal_script_to_builder(
        &self,
        mut builder: ScriptBuilder,
    ) -> Result<ScriptBuilder, Box<dyn Error>> {
        builder = builder
            .push_opcode(opcodes::OP_FALSE)
            .push_opcode(opcodes::all::OP_IF)
            .push_slice(constants::PROTOCOL_ID);

        Self::append(
            constants::CONTENT_TYPE_TAG,
            &mut builder,
            &self.content_type,
        );
        Self::append(
            constants::CONTENT_ENCODING_TAG,
            &mut builder,
            &self.content_encoding,
        );
        Self::append(
            constants::METAPROTOCOL_TAG,
            &mut builder,
            &self.metaprotocol,
        );
        Self::append_array(constants::PARENT_TAG, &mut builder, &self.parents);
        Self::append(constants::DELEGATE_TAG, &mut builder, &self.delegate);
        Self::append(constants::POINTER_TAG, &mut builder, &self.pointer);
        Self::append(constants::METADATA_TAG, &mut builder, &self.metadata);
        Self::append(constants::RUNE_TAG, &mut builder, &self.rune);

        if let Some(body) = &self.body {
            builder = builder.push_slice(constants::BODY_TAG);
            for chunk in body.chunks(MAX_SCRIPT_ELEMENT_SIZE) {
                builder = builder.push_slice::<&PushBytes>(chunk.try_into().unwrap());
            }
        }

        Ok(builder.push_opcode(opcodes::all::OP_ENDIF))
    }

    fn append(tag: [u8; 1], builder: &mut ScriptBuilder, value: &Option<Vec<u8>>) {
        if let Some(value) = value {
            let mut tmp = ScriptBuilder::new();
            mem::swap(&mut tmp, builder);

            if is_chunked(tag) {
                for chunk in value.chunks(MAX_SCRIPT_ELEMENT_SIZE) {
                    tmp = tmp
                        .push_slice::<&PushBytes>(tag.as_slice().try_into().unwrap())
                        .push_slice::<&PushBytes>(chunk.try_into().unwrap());
                }
            } else {
                tmp = tmp
                    .push_slice::<&PushBytes>(tag.as_slice().try_into().unwrap())
                    .push_slice::<&PushBytes>(value.as_slice().try_into().unwrap());
            }

            mem::swap(&mut tmp, builder);
        }
    }

    fn append_array(tag: [u8; 1], builder: &mut ScriptBuilder, values: &Vec<Vec<u8>>) {
        let mut tmp = ScriptBuilder::new();
        mem::swap(&mut tmp, builder);

        for value in values {
            tmp = tmp
                .push_slice::<&PushBytes>(tag.as_slice().try_into().unwrap())
                .push_slice::<&PushBytes>(value.as_slice().try_into().unwrap());
        }

        mem::swap(&mut tmp, builder);
    }

    fn validate_content_type(&self) -> Result<Self, Box<dyn Error>> {
        if let Some(content_type) = &self.content_type {
            let content_type_str =
                std::str::from_utf8(content_type).map_err(|_| "Invalid UTF-8 encoding")?;
            if !content_type_str.contains('/') {
                return Err("Invalid content type".into());
            }
        }

        Ok(self.clone())
    }

    pub fn from_json_str(data: &str) -> Result<Self, Box<dyn Error>> {
        Self::from_str(data)?.validate_content_type()
    }

    pub fn as_push_bytes(&self) -> Result<PushBytesBuf, Box<dyn Error>> {
        bytes_to_push_bytes(
            to_string(self)
                .map_err(|_| Box::<dyn Error>::from("Failed to serialize to JSON"))?
                .as_bytes(),
        )
    }

    pub fn body(&self) -> Option<&str> {
        std::str::from_utf8(self.body.as_ref()?).ok()
    }

    pub fn content_type(&self) -> Option<&str> {
        std::str::from_utf8(self.content_type.as_ref()?).ok()
    }

    pub fn metadata(&self) -> Option<ciborium::Value> {
        ciborium::from_reader(Cursor::new(self.metadata.as_ref()?)).ok()
    }

    pub fn reveal_script_as_scriptbuf(
        &self,
        builder: ScriptBuilder,
    ) -> Result<ScriptBuf, Box<dyn Error>> {
        Ok(self.append_reveal_script_to_builder(builder)?.into_script())
    }
}

impl FromStr for InscriptionData {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s).map_err(|e| e.into())
    }
}

fn is_chunked(tag: [u8; 1]) -> bool {
    matches!(tag, constants::METADATA_TAG)
}
