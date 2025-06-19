#![no_std]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(generic_arg_infer)]
#![feature(type_alias_impl_trait)]
#![feature(let_chains)]
#![feature(try_blocks)]
#![feature(impl_trait_in_assoc_type)]
#![feature(iter_collect_into)]
extern crate alloc;

mod matrix_spi;
pub mod matrix_parl_io;
pub mod config;
pub mod network;
mod net_utils;
mod captive;