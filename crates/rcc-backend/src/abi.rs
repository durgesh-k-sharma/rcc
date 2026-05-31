//! System V AMD64 calling convention for the rcc backend.
//!
//! This module defines the contract between all function calls:
//! argument classification, register assignment, stack layout, and
//! callee/caller-saved register conventions.

/// x86-64 general-purpose registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reg {
    RAX, RBX, RCX, RDX,
    RSI, RDI, RBP, RSP,
    R8, R9, R10, R11, R12, R13, R14, R15,
}

impl Reg {
    /// The name of this register as used in assembly text.
    pub fn name(&self) -> &'static str {
        match self {
            Reg::RAX => "%rax",
            Reg::RBX => "%rbx",
            Reg::RCX => "%rcx",
            Reg::RDX => "%rdx",
            Reg::RSI => "%rsi",
            Reg::RDI => "%rdi",
            Reg::RBP => "%rbp",
            Reg::RSP => "%rsp",
            Reg::R8  => "%r8",
            Reg::R9  => "%r9",
            Reg::R10 => "%r10",
            Reg::R11 => "%r11",
            Reg::R12 => "%r12",
            Reg::R13 => "%r13",
            Reg::R14 => "%r14",
            Reg::R15 => "%r15",
        }
    }

    /// The 32-bit (dword) encoding of this register for use in instructions.
    pub fn name_32(&self) -> &'static str {
        match self {
            Reg::RAX => "%eax",
            Reg::RBX => "%ebx",
            Reg::RCX => "%ecx",
            Reg::RDX => "%edx",
            Reg::RSI => "%esi",
            Reg::RDI => "%edi",
            Reg::RBP => "%ebp",
            Reg::RSP => "%esp",
            Reg::R8  => "%r8d",
            Reg::R9  => "%r9d",
            Reg::R10 => "%r10d",
            Reg::R11 => "%r11d",
            Reg::R12 => "%r12d",
            Reg::R13 => "%r13d",
            Reg::R14 => "%r14d",
            Reg::R15 => "%r15d",
        }
    }
}

/// The classification of an argument under the calling convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgClass {
    /// Passed in an integer register (or on the stack once registers are exhausted).
    INTEGER,
    /// Passed in an SSE register (not yet implemented).
    SSE,
    /// Passed indirectly on the stack via a pointer (MEMORY class in SysV terms).
    MEMORY,
}

/// A minimal type representation for ABI argument classification.
///
/// This will be superseded by the full type system once the frontend is built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbiType {
    /// A signed or unsigned integer of the given bit width.
    Integer(u32),
    /// A pointer (treated as 64-bit integer for classification).
    Pointer,
    /// A floating-point type of the given bit width.
    Float(u32),
    /// A struct/union/array type with the given total size in bytes.
    Aggregate(u32),
}

impl AbiType {
    /// The size of this type in bytes.
    pub fn size(&self) -> u32 {
        match self {
            AbiType::Integer(bits) => (*bits + 7) / 8,
            AbiType::Pointer => 8,
            AbiType::Float(bits) => (*bits + 7) / 8,
            AbiType::Aggregate(size) => *size,
        }
    }
}

/// The calling convention interface.
///
/// Implementations define how arguments are classified, which registers
/// are used for passing arguments and return values, and the stack
/// layout conventions.
pub trait CallingConvention {
    /// Classify an argument of the given type.
    fn classify_argument(&self, ty: &AbiType) -> ArgClass;

    /// Return the register for the `index`-th integer-class argument
    /// (0-based), or `None` if arguments at this position go on the stack.
    fn argument_register(&self, index: usize) -> Option<Reg>;

    /// The register used for integer-class return values.
    fn return_register(&self) -> Reg;

    /// The list of registers that must be preserved across function calls.
    fn callee_saved_registers(&self) -> &[Reg];

    /// The list of registers that the caller must save if needed.
    fn caller_saved_registers(&self) -> &[Reg];

    /// The required stack alignment at the point of a `call` instruction (in bytes).
    fn stack_alignment(&self) -> u32;

    /// The size of the red zone below `rsp` (in bytes).
    fn red_zone_size(&self) -> u32;

    /// The number of integer argument registers available before spilling to stack.
    fn num_integer_arg_registers(&self) -> usize;
}

// ---------------------------------------------------------------------------
// System V AMD64 implementation
// ---------------------------------------------------------------------------

/// The System V AMD64 calling convention for x86-64 Linux.
///
/// References:
/// - System V Application Binary Interface: AMD64 Architecture Processor Supplement
///   (https://raw.githubusercontent.com/hjl-tools/x86-psABI/main/x86-64-psABI.pdf)
pub struct SysVAmd64;

impl CallingConvention for SysVAmd64 {
    fn classify_argument(&self, ty: &AbiType) -> ArgClass {
        match ty {
            // Integers and pointers → INTEGER class
            AbiType::Integer(_) | AbiType::Pointer => ArgClass::INTEGER,
            // Floats → SSE class
            AbiType::Float(_) => ArgClass::SSE,
            // Aggregates larger than 16 bytes → MEMORY class
            // Aggregates 16 bytes or smaller that fit in two eightbytes → INTEGER
            AbiType::Aggregate(size) => {
                if *size > 16 {
                    ArgClass::MEMORY
                } else {
                    ArgClass::INTEGER
                }
            }
        }
    }

    fn argument_register(&self, index: usize) -> Option<Reg> {
        match index {
            0 => Some(Reg::RDI),
            1 => Some(Reg::RSI),
            2 => Some(Reg::RDX),
            3 => Some(Reg::RCX),
            4 => Some(Reg::R8),
            5 => Some(Reg::R9),
            _ => None, // stack spill
        }
    }

    fn return_register(&self) -> Reg {
        Reg::RAX
    }

    fn callee_saved_registers(&self) -> &[Reg] {
        &[
            Reg::RBX, Reg::RBP,
            Reg::R12, Reg::R13, Reg::R14, Reg::R15,
        ]
    }

    fn caller_saved_registers(&self) -> &[Reg] {
        &[
            Reg::RAX, Reg::RCX, Reg::RDX,
            Reg::RSI, Reg::RDI,
            Reg::R8,  Reg::R9,  Reg::R10, Reg::R11,
        ]
    }

    fn stack_alignment(&self) -> u32 {
        16
    }

    fn red_zone_size(&self) -> u32 {
        128
    }

    fn num_integer_arg_registers(&self) -> usize {
        6
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_integer_as_integer() {
        let abi = SysVAmd64;
        assert_eq!(abi.classify_argument(&AbiType::Integer(32)), ArgClass::INTEGER);
        assert_eq!(abi.classify_argument(&AbiType::Integer(8)), ArgClass::INTEGER);
        assert_eq!(abi.classify_argument(&AbiType::Integer(64)), ArgClass::INTEGER);
    }

    #[test]
    fn classify_pointer_as_integer() {
        let abi = SysVAmd64;
        assert_eq!(abi.classify_argument(&AbiType::Pointer), ArgClass::INTEGER);
    }

    #[test]
    fn classify_float_as_sse() {
        let abi = SysVAmd64;
        assert_eq!(abi.classify_argument(&AbiType::Float(32)), ArgClass::SSE);
        assert_eq!(abi.classify_argument(&AbiType::Float(64)), ArgClass::SSE);
    }

    #[test]
    fn classify_small_aggregate_as_integer() {
        let abi = SysVAmd64;
        // 8-byte struct → fits in one register
        assert_eq!(abi.classify_argument(&AbiType::Aggregate(8)), ArgClass::INTEGER);
        // 16-byte struct → two eightbytes, integer class
        assert_eq!(abi.classify_argument(&AbiType::Aggregate(16)), ArgClass::INTEGER);
    }

    #[test]
    fn classify_large_aggregate_as_memory() {
        let abi = SysVAmd64;
        assert_eq!(abi.classify_argument(&AbiType::Aggregate(17)), ArgClass::MEMORY);
        assert_eq!(abi.classify_argument(&AbiType::Aggregate(32)), ArgClass::MEMORY);
    }

    #[test]
    fn argument_registers_correct() {
        let abi = SysVAmd64;
        assert_eq!(abi.argument_register(0), Some(Reg::RDI));
        assert_eq!(abi.argument_register(1), Some(Reg::RSI));
        assert_eq!(abi.argument_register(2), Some(Reg::RDX));
        assert_eq!(abi.argument_register(3), Some(Reg::RCX));
        assert_eq!(abi.argument_register(4), Some(Reg::R8));
        assert_eq!(abi.argument_register(5), Some(Reg::R9));
        assert_eq!(abi.argument_register(6), None);
    }

    #[test]
    fn return_register_is_rax() {
        let abi = SysVAmd64;
        assert_eq!(abi.return_register(), Reg::RAX);
    }

    #[test]
    fn callee_saved_registers_count() {
        let abi = SysVAmd64;
        assert_eq!(abi.callee_saved_registers().len(), 6);
    }

    #[test]
    fn caller_saved_registers_count() {
        let abi = SysVAmd64;
        assert_eq!(abi.caller_saved_registers().len(), 9);
    }

    #[test]
    fn no_overlap_in_saved_lists() {
        let abi = SysVAmd64;
        for callee in abi.callee_saved_registers() {
            assert!(
                !abi.caller_saved_registers().contains(callee),
                "register {:?} appears in both caller-saved and callee-saved lists",
                callee,
            );
        }
    }

    #[test]
    fn stack_alignment_is_16_bytes() {
        let abi = SysVAmd64;
        assert_eq!(abi.stack_alignment(), 16);
    }

    #[test]
    fn red_zone_is_128_bytes() {
        let abi = SysVAmd64;
        assert_eq!(abi.red_zone_size(), 128);
    }

    #[test]
    fn abi_type_size() {
        assert_eq!(AbiType::Integer(32).size(), 4);
        assert_eq!(AbiType::Integer(8).size(), 1);
        assert_eq!(AbiType::Integer(64).size(), 8);
        assert_eq!(AbiType::Pointer.size(), 8);
        assert_eq!(AbiType::Float(32).size(), 4);
        assert_eq!(AbiType::Float(64).size(), 8);
        assert_eq!(AbiType::Aggregate(12).size(), 12);
    }

    #[test]
    fn register_names() {
        assert_eq!(Reg::RAX.name(), "%rax");
        assert_eq!(Reg::R15.name(), "%r15");
        assert_eq!(Reg::R8.name_32(), "%r8d");
        assert_eq!(Reg::RDI.name_32(), "%edi");
    }
}
