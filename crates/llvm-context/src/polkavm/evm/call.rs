//! Translates a contract call.

use inkwell::values::BasicValue;

use crate::polkavm::context::argument::Argument;
use crate::polkavm::context::Context;
use crate::polkavm::Dependency;

const STATIC_CALL_FLAG: u32 = 0b0001_0000;
const REENTRANT_CALL_FLAG: u32 = 0b0000_1000;

/// Translates a contract call.
#[allow(clippy::too_many_arguments)]
pub fn call<'ctx, D>(
    context: &mut Context<'ctx, D>,
    gas: inkwell::values::IntValue<'ctx>,
    address: inkwell::values::IntValue<'ctx>,
    value: Option<inkwell::values::IntValue<'ctx>>,
    input_offset: inkwell::values::IntValue<'ctx>,
    input_length: inkwell::values::IntValue<'ctx>,
    output_offset: inkwell::values::IntValue<'ctx>,
    output_length: inkwell::values::IntValue<'ctx>,
    _constants: Vec<Option<num::BigUint>>,
    static_call: bool,
) -> anyhow::Result<inkwell::values::BasicValueEnum<'ctx>>
where
    D: Dependency + Clone,
{
    let address_pointer = context.build_address_argument_store(address)?;

    let value = value.unwrap_or_else(|| context.word_const(0));
    let value_pointer = context.build_alloca_at_entry(context.word_type(), "value_pointer");
    context.build_store(value_pointer, value)?;

    let input_offset = context.safe_truncate_int_to_xlen(input_offset)?;
    let input_length = context.safe_truncate_int_to_xlen(input_length)?;
    let output_offset = context.safe_truncate_int_to_xlen(output_offset)?;
    let output_length = context.safe_truncate_int_to_xlen(output_length)?;

    // TODO: What to supply here? Is there a weight to gas?
    let _gas = context
        .builder()
        .build_int_truncate(gas, context.integer_type(64), "gas")?;

    let input_pointer = context.build_heap_gep(input_offset, input_length)?;
    let output_pointer = context.build_heap_gep(output_offset, output_length)?;

    let output_length_pointer = context.build_alloca_at_entry(context.xlen_type(), "output_length");
    context.build_store(output_length_pointer, output_length)?;

    let deposit_pointer = context.build_alloca_at_entry(context.word_type(), "deposit_pointer");
    context.build_store(deposit_pointer, context.word_type().const_all_ones())?;

    let flags = if static_call {
        REENTRANT_CALL_FLAG | STATIC_CALL_FLAG
    } else {
        REENTRANT_CALL_FLAG
    };
    let flags = context.xlen_type().const_int(flags as u64, false);

    let argument_type = revive_runtime_api::calling_convention::call(context.llvm());
    let argument_pointer = context.build_alloca_at_entry(argument_type, "call_arguments");
    let arguments = &[
        flags.as_basic_value_enum(),
        address_pointer.value.as_basic_value_enum(),
        context
            .integer_const(revive_common::BIT_LENGTH_X64, u64::MAX)
            .as_basic_value_enum(),
        context
            .integer_const(revive_common::BIT_LENGTH_X64, u64::MAX)
            .as_basic_value_enum(),
        deposit_pointer.value.as_basic_value_enum(),
        value_pointer.value.as_basic_value_enum(),
        input_pointer.value.as_basic_value_enum(),
        input_length.as_basic_value_enum(),
        output_pointer.value.as_basic_value_enum(),
        output_length_pointer.value.as_basic_value_enum(),
    ];
    revive_runtime_api::calling_convention::spill(
        context.builder(),
        argument_pointer.value,
        argument_type,
        arguments,
    )?;

    let name = revive_runtime_api::polkavm_imports::CALL;
    let argument_pointer = context.builder().build_ptr_to_int(
        argument_pointer.value,
        context.xlen_type(),
        "call_argument_pointer",
    )?;
    let success = context
        .build_runtime_call(name, &[argument_pointer.into()])
        .unwrap_or_else(|| panic!("{name} should return a value"))
        .into_int_value();

    let is_success = context.builder().build_int_compare(
        inkwell::IntPredicate::EQ,
        success,
        context.integer_const(revive_common::BIT_LENGTH_X64, 0),
        "is_success",
    )?;

    Ok(context
        .builder()
        .build_int_z_extend(is_success, context.word_type(), "success")?
        .as_basic_value_enum())
}

#[allow(clippy::too_many_arguments)]
pub fn delegate_call<'ctx, D>(
    context: &mut Context<'ctx, D>,
    gas: inkwell::values::IntValue<'ctx>,
    address: inkwell::values::IntValue<'ctx>,
    input_offset: inkwell::values::IntValue<'ctx>,
    input_length: inkwell::values::IntValue<'ctx>,
    output_offset: inkwell::values::IntValue<'ctx>,
    output_length: inkwell::values::IntValue<'ctx>,
    _constants: Vec<Option<num::BigUint>>,
) -> anyhow::Result<inkwell::values::BasicValueEnum<'ctx>>
where
    D: Dependency + Clone,
{
    let address_pointer = context.build_address_argument_store(address)?;

    let input_offset = context.safe_truncate_int_to_xlen(input_offset)?;
    let input_length = context.safe_truncate_int_to_xlen(input_length)?;
    let output_offset = context.safe_truncate_int_to_xlen(output_offset)?;
    let output_length = context.safe_truncate_int_to_xlen(output_length)?;

    // TODO: What to supply here? Is there a weight to gas?
    let _gas = context
        .builder()
        .build_int_truncate(gas, context.integer_type(64), "gas")?;

    let input_pointer = context.build_heap_gep(input_offset, input_length)?;
    let output_pointer = context.build_heap_gep(output_offset, output_length)?;

    let output_length_pointer = context.build_alloca_at_entry(context.xlen_type(), "output_length");
    context.build_store(output_length_pointer, output_length)?;

    let deposit_pointer = context.build_alloca_at_entry(context.word_type(), "deposit_pointer");
    context.build_store(deposit_pointer, context.word_type().const_all_ones())?;

    let flags = context.xlen_type().const_int(0u64, false);

    let argument_type = revive_runtime_api::calling_convention::delegate_call(context.llvm());
    let argument_pointer = context.build_alloca_at_entry(argument_type, "delegate_call_arguments");
    let arguments = &[
        flags.as_basic_value_enum(),
        address_pointer.value.as_basic_value_enum(),
        context
            .integer_const(revive_common::BIT_LENGTH_X64, u64::MAX)
            .as_basic_value_enum(),
        context
            .integer_const(revive_common::BIT_LENGTH_X64, u64::MAX)
            .as_basic_value_enum(),
        deposit_pointer.value.as_basic_value_enum(),
        input_pointer.value.as_basic_value_enum(),
        input_length.as_basic_value_enum(),
        output_pointer.value.as_basic_value_enum(),
        output_length_pointer.value.as_basic_value_enum(),
    ];
    revive_runtime_api::calling_convention::spill(
        context.builder(),
        argument_pointer.value,
        argument_type,
        arguments,
    )?;

    let name = revive_runtime_api::polkavm_imports::DELEGATE_CALL;
    let argument_pointer = context.builder().build_ptr_to_int(
        argument_pointer.value,
        context.xlen_type(),
        "delegate_call_argument_pointer",
    )?;
    let success = context
        .build_runtime_call(name, &[argument_pointer.into()])
        .unwrap_or_else(|| panic!("{name} should return a value"))
        .into_int_value();

    let is_success = context.builder().build_int_compare(
        inkwell::IntPredicate::EQ,
        success,
        context.integer_const(revive_common::BIT_LENGTH_X64, 0),
        "is_success",
    )?;

    Ok(context
        .builder()
        .build_int_z_extend(is_success, context.word_type(), "success")?
        .as_basic_value_enum())
}

/// Translates the Yul `linkersymbol` instruction.
pub fn linker_symbol<'ctx, D>(
    context: &mut Context<'ctx, D>,
    mut arguments: [Argument<'ctx>; 1],
) -> anyhow::Result<inkwell::values::BasicValueEnum<'ctx>>
where
    D: Dependency + Clone,
{
    let path = arguments[0]
        .original
        .take()
        .ok_or_else(|| anyhow::anyhow!("Linker symbol literal is missing"))?;

    Ok(context
        .resolve_library(path.as_str())?
        .as_basic_value_enum())
}
