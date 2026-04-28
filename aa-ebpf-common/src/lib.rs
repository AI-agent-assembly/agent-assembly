//! Shared types between eBPF kernel-space probes and userspace loader.
//!
//! All types in this crate use fixed-size representations compatible with
//! eBPF maps. This crate is `no_std` so it can be compiled for both the
//! `bpfel-unknown-none` BPF target and standard userspace targets.

#![no_std]
