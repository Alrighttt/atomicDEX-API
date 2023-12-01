#![cfg(feature = "python_tests")]

extern crate cpython;
#[macro_use]
extern crate quickcheck;
extern crate zbase32;

mod common;
use common::*;

use cpython::{Python, PyDict, PyResult, PyBytes};

quickcheck! {
    fn encode(input: Vec<u8>) -> bool {
        let bits = rand_bit_length(input.len(), 8);
        let rust = zbase32::encode(&input, bits);
        let python = py_encode(&input[..], Some(bits)).unwrap();
        rust == python
    }
}

quickcheck! {
    fn decode(input: ZBaseEncodedData) -> bool {
        let bits = rand_bit_length(input.len(), 5);
        let rust = zbase32::decode(input.as_bytes(), bits).unwrap();
        let python = py_decode(&input.as_bytes()[..], Some(bits)).unwrap();
        rust == python
    }
}

quickcheck! {
    fn encode_bytes(input: Vec<u8>) -> bool {
        let rust = zbase32::encode_full_bytes(&input);
        let python = py_encode(&input[..], None).unwrap();
        rust == python
    }
}

quickcheck! {
    fn decode_bytes(input: ZBaseEncodedData) -> bool {
        let rust = zbase32::decode_full_bytes(input.as_bytes()).unwrap();
        let python = py_decode(&input.as_bytes()[..], None).unwrap();
        rust == python
    }
}

fn py_encode(input: &[u8], bits: Option<u64>) -> PyResult<String> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    let locals = PyDict::new(py);
    locals.set_item(py, "pyzbase32", py.import("pyzbase32")?)?;
    locals.set_item(py, "input", PyBytes::new(py, input))?;
    let result: String = if let Some(bits) = bits {
        locals.set_item(py, "bits", bits)?;
        py.eval(
            "pyzbase32.encode(input, bits).decode()",
            None,
            Some(&locals),
        )
    } else {
        py.eval(
            "pyzbase32.encode_bytes(input).decode()",
            None,
            Some(&locals),
        )
    }?
        .extract(py)?;
    Ok(result)
}

fn py_decode(input: &[u8], bits: Option<u64>) -> PyResult<Vec<u8>> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    let locals = PyDict::new(py);
    locals.set_item(py, "pyzbase32", py.import("pyzbase32")?)?;
    locals.set_item(py, "input", PyBytes::new(py, input))?;
    let result: PyBytes = if let Some(bits) = bits {
        locals.set_item(py, "bits", bits)?;
        py.eval("pyzbase32.decode(input, bits)", None, Some(&locals))
    } else {
        py.eval("pyzbase32.decode_bytes(input)", None, Some(&locals))
    }?
        .extract(py)?;
    Ok(Vec::from(result.data(py)))
}
