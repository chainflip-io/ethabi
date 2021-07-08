// Copyright 2015-2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Function param.

use crate::ParamType;
#[cfg(feature = "std")]
use crate::TupleParam;
#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(feature = "std")]
use serde::{
	de::{Error, MapAccess, Visitor},
	Deserialize, Deserializer,
};
#[cfg(feature = "std")]
use std::fmt;

/// Function param.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
	/// Param name.
	pub name: String,
	/// Param type.
	pub kind: ParamType,
}

impl Param {
	/// Construct a Param working around the String limitations of Substrate
	pub fn new(new_name: &str, kind: ParamType) -> Self {
		Param {
			name: new_name.into(),
			kind,
		}
	}
}
// no_std friendly coming from Substrate
impl From<(&str, ParamType)> for Param {
	fn from(param: (&str, ParamType)) -> Self {
		Param {
			name: param.0.into(),
			kind: param.1,
		}
	}
}

#[cfg(feature = "std")]
impl<'a> Deserialize<'a> for Param {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'a>,
	{
		deserializer.deserialize_any(ParamVisitor)
	}
}

#[cfg(feature = "std")]
struct ParamVisitor;

#[cfg(feature = "std")]
impl<'a> Visitor<'a> for ParamVisitor {
	type Value = Param;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		write!(formatter, "a valid event parameter spec")
	}

	fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
	where
		V: MapAccess<'a>,
	{
		let mut name = None;
		let mut kind = None;
		let mut components = None;

		while let Some(ref key) = map.next_key::<String>()? {
			match key.as_ref() {
				"name" => {
					if name.is_some() {
						return Err(Error::duplicate_field("name"));
					}
					name = Some(map.next_value()?);
				}
				"type" => {
					if kind.is_some() {
						return Err(Error::duplicate_field("kind"));
					}
					kind = Some(map.next_value()?);
				}
				"components" => {
					if components.is_some() {
						return Err(Error::duplicate_field("components"));
					}
					let component: Vec<TupleParam> = map.next_value()?;
					components = Some(component)
				}
				_ => {}
			}
		}
		let name = name.ok_or_else(|| Error::missing_field("name"))?;
		let kind =
			kind.ok_or_else(|| Error::missing_field("kind")).and_then(|param_type: ParamType| match param_type {
				ParamType::Tuple(_) => {
					let tuple_params = components.ok_or_else(|| Error::missing_field("components"))?;
					Ok(ParamType::Tuple(tuple_params.into_iter().map(|param| param.kind).collect()))
				}
				ParamType::Array(inner_param_type) => match *inner_param_type {
					ParamType::Tuple(_) => {
						let tuple_params = components.ok_or_else(|| Error::missing_field("components"))?;
						Ok(ParamType::Array(Box::new(ParamType::Tuple(
							tuple_params.into_iter().map(|param| param.kind).collect(),
						))))
					}
					_ => Ok(ParamType::Array(inner_param_type)),
				},
				ParamType::FixedArray(inner_param_type, size) => match *inner_param_type {
					ParamType::Tuple(_) => {
						let tuple_params = components.ok_or_else(|| Error::missing_field("components"))?;
						Ok(ParamType::FixedArray(
							Box::new(ParamType::Tuple(tuple_params.into_iter().map(|param| param.kind).collect())),
							size,
						))
					}
					_ => Ok(ParamType::FixedArray(inner_param_type, size)),
				},
				_ => Ok(param_type),
			})?;
		Ok(Param { name, kind })
	}
}

#[cfg(all(test, feature = "std"))]
mod tests {
	use crate::{Param, ParamType};

	#[test]
	fn param_deserialization() {
		let s = r#"{
			"name": "foo",
			"type": "address"
		}"#;

		let deserialized: Param = serde_json::from_str(s).unwrap();

		assert_eq!(deserialized, Param { name: "foo".to_owned(), kind: ParamType::Address });
	}

	#[test]
	fn param_tuple_deserialization() {
		let s = r#"{
			"name": "foo",
			"type": "tuple",
			"components": [
				{
					"name": "amount",
					"type": "uint48"
				},
				{
					"name": "things",
					"type": "tuple",
					"components": [
						{
							"name": "baseTupleParam",
							"type": "address"
						}
					]
				}
			]
		}"#;

		let deserialized: Param = serde_json::from_str(s).unwrap();

		assert_eq!(
			deserialized,
			Param {
				name: "foo".to_owned(),
				kind: ParamType::Tuple(vec![ParamType::Uint(48), ParamType::Tuple(vec![ParamType::Address])]),
			}
		);
	}

	#[test]
	fn param_tuple_array_deserialization() {
		let s = r#"{
			"name": "foo",
			"type": "tuple[]",
			"components": [
				{
					"name": "amount",
					"type": "uint48"
				},
				{
					"name": "to",
					"type": "address"
				},
				{
					"name": "from",
					"type": "address"
				}
			]
		}"#;

		let deserialized: Param = serde_json::from_str(s).unwrap();

		assert_eq!(
			deserialized,
			Param {
				name: "foo".to_owned(),
				kind: ParamType::Array(Box::new(ParamType::Tuple(vec![
					ParamType::Uint(48),
					ParamType::Address,
					ParamType::Address
				]))),
			}
		);
	}

	#[test]
	fn param_tuple_fixed_array_deserialization() {
		let s = r#"{
			"name": "foo",
			"type": "tuple[2]",
			"components": [
				{
					"name": "amount",
					"type": "uint48"
				},
				{
					"name": "to",
					"type": "address"
				},
				{
					"name": "from",
					"type": "address"
				}
			]
		}"#;

		let deserialized: Param = serde_json::from_str(s).unwrap();

		assert_eq!(
			deserialized,
			Param {
				name: "foo".to_owned(),
				kind: ParamType::FixedArray(
					Box::new(ParamType::Tuple(vec![ParamType::Uint(48), ParamType::Address, ParamType::Address])),
					2
				),
			}
		);
	}
}
