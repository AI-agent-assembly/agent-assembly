//! Python FFI bindings for Agent Assembly via PyO3.
//!
//! This crate exposes the Agent Assembly SDK to Python. It compiles to a
//! `cdylib` Python extension module, allowing Python agents to instrument
//! themselves with the governance shim without leaving the Python runtime.

mod config;
