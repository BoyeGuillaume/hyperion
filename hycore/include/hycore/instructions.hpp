#pragma once

#include <hycore/except.h>
#include <hycore/types.hpp>
#include <cstdint>
#include <optional>

namespace hycore::instructions
{
    using Name = std::uint32_t;

    enum class CallingConvention : std::uint8_t
    {
        CDecl,
        Fast,
        Cold,
        GHC,
        CC11,
        AnyReg,
        PreserveMost,
        PreserveAll,
        PreserveNone,
        CxxFastTls,
        Tail,
        SwiftTail,
        CfGuardCheck,
    };

    enum class Visibility : std::uint8_t
    {
        Default,
        Hidden,
        Protected,
    };

    enum class OverflowBehavior : std::uint8_t
    {
        Wrap,
        Saturate,
        Trap,
    };

    using Operand = std::variant<Name, constants::Constant>;

    struct AddI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
        OverflowBehavior overflowBehavior;
    };

    struct SubI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
        OverflowBehavior overflowBehavior;
    };

    struct MulI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
        OverflowBehavior overflowBehavior;
    };

    struct DivI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
        bool isSigned;
    };

    struct RemI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
        bool isSigned;
    };

    struct AndI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
    };

    struct OrI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
    };

    struct XorI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
    };

    enum class ShiftType : std::uint8_t
    {
        LogicalLeft,
        LogicalRight,
        ArithmeticRight,
    };

    struct ShiftI
    {
        Operand value;
        Operand shiftAmount;
        Name dest;
        ShiftType shiftType;
    };

    struct FpAddI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
    };

    struct FpSubI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
    };

    struct FpMulI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
    };

    struct FpDivI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
    };

    struct FpRemI
    {
        Operand lhs;
        Operand rhs;
        Name dest;
    };

    enum class MemoryOrdering : std::uint8_t
    {
        Unordered,
        Monotonic,
        Acq,
        Rel,
        AcqRel,
        SeqCst,
    };

    struct MemLoadI
    {
        Operand address;
        TypeId loadType;
        Name dest;
        std::uint32_t alignment;
        std::optional<MemoryOrdering> ordering;
        bool isVolatile;
    };

    struct MemStoreI
    {
        Operand address;
        Operand value;
        TypeId valueType;
        std::uint32_t alignment;
        std::optional<MemoryOrdering> ordering;
        bool isVolatile;
    };

    struct MemAllocaI
    {
        TypeId allocatedType;
        Operand elementCount;
        Name dest;
        std::uint32_t alignment;
    };

    struct GetElemPtrI
    {
        Operand baseAddress;
        TypeId baseType;
        std::vector<Operand> indices;
        Name dest;
    };

    using Instruction = std::variant<AddI,
                                     SubI,
                                     MulI,
                                     DivI,
                                     RemI,
                                     AndI,
                                     OrI,
                                     XorI,
                                     ShiftI,
                                     FpAddI,
                                     FpSubI,
                                     FpMulI,
                                     FpDivI,
                                     FpRemI,
                                     MemLoadI,
                                     MemStoreI,
                                     MemAllocaI,
                                     GetElemPtrI>;
}
