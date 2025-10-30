#pragma once

#include <hycore/except.h>
#include <cstdint>
#include <vector>
#include <stdexcept>
#include <memory>
#include <variant>

namespace hycore
{
    using TypeId = std::uint32_t;
}

namespace hycore::types
{
    struct VoidT
    {
        inline std::strong_ordering operator<=>(const VoidT &) const noexcept
        {
            return std::strong_ordering::equal;
        }
    };

    struct LabelT
    {
        inline std::strong_ordering operator<=>(const LabelT &) const noexcept
        {
            return std::strong_ordering::equal;
        }
    };

    struct FunctionT
    {
        TypeId returnType;
        std::vector<TypeId> parameterTypes;

        inline std::strong_ordering operator<=>(const FunctionT &other) const noexcept
        {
            if (auto cmp = returnType <=> other.returnType; cmp != 0)
                return cmp;
            return parameterTypes <=> other.parameterTypes;
        }
    };

    struct IntegerT
    {
        std::uint16_t bitWidth;

        inline std::strong_ordering operator<=>(const IntegerT &other) const noexcept
        {
            return bitWidth <=> other.bitWidth;
        }
    };

    struct PointerT
    {
        TypeId pointeeType;

        inline std::strong_ordering operator<=>(const PointerT &other) const noexcept
        {
            return pointeeType <=> other.pointeeType;
        }
    };

    struct VectorT
    {
        TypeId elementType;
        std::uint32_t elementCount;
        bool isScalable{false};

        inline std::strong_ordering operator<=>(const VectorT &other) const noexcept
        {
            if (auto cmp = elementType <=> other.elementType; cmp != 0)
                return cmp;
            if (auto cmp = elementCount <=> other.elementCount; cmp != 0)
                return cmp;
            return isScalable <=> other.isScalable;
        }
    };

    enum class FloatingPointT
    {
        Fp16,
        Bf16,
        Fp32,
        Fp64,
        Fp128,
        X86Fp80,
        PpcFp128,
    };

    static const IntegerT i1 = {1};
    static const IntegerT i8 = {8};
    static const IntegerT i16 = {16};
    static const IntegerT i32 = {32};
    static const IntegerT i64 = {64};

    using PrimitiveT = std::variant<VoidT,
                                    LabelT,
                                    FunctionT,
                                    IntegerT,
                                    PointerT,
                                    VectorT,
                                    FloatingPointT>;

    inline std::strong_ordering operator<=>(const PrimitiveT &lhs, const PrimitiveT &rhs) noexcept
    {
        if (auto cmp = lhs.index() <=> rhs.index(); cmp != 0)
            return cmp;
        return std::visit([](const auto &l, const auto &r) {
            if constexpr (std::is_same_v<std::decay_t<decltype(l)>, std::decay_t<decltype(r)>>)
                return l <=> r;
            else {
                return std::strong_ordering::equal; // Should not happen
            }
        }, lhs, rhs);
    }

    inline bool operator==(const PrimitiveT &lhs, const PrimitiveT &rhs) noexcept
    {
        return (lhs <=> rhs) == std::strong_ordering::equal;
    }

    inline bool operator!=(const PrimitiveT &lhs, const PrimitiveT &rhs) noexcept
    {
        return !(lhs == rhs);
    }

    struct StructT
    {
        std::vector<TypeId> memberTypes;

        inline std::strong_ordering operator<=>(const StructT &other) const noexcept
        {
            return memberTypes <=> other.memberTypes;
        }
    };

    using Type = std::variant<VoidT,
                              LabelT,
                              FunctionT,
                              IntegerT,
                              PointerT,
                              VectorT,
                              FloatingPointT,
                              StructT>;

    inline std::strong_ordering operator<=>(const Type &lhs, const Type &rhs) noexcept
    {
        if (auto cmp = lhs.index() <=> rhs.index(); cmp != 0)
            return cmp;
        return std::visit([](const auto &l, const auto &r) {
            if constexpr (std::is_same_v<std::decay_t<decltype(l)>, std::decay_t<decltype(r)>>)
                return l <=> r;
            else {
                return std::strong_ordering::equal; // Should not happen
            }
        }, lhs, rhs);
    }

    inline bool operator==(const Type &lhs, const Type &rhs) noexcept
    {
        return (lhs <=> rhs) == std::strong_ordering::equal;
    }

    inline bool operator!=(const Type &lhs, const Type &rhs) noexcept
    {
        return !(lhs == rhs);
    }

    class TypeRegistry
    {
    public:
        HYCORE_API explicit TypeRegistry();
        HYCORE_API ~TypeRegistry();

        HYCORE_API const Type& get(TypeId typeId) const;
        HYCORE_API TypeId getOrInsert(Type &&type);
        HYCORE_API size_t size() const noexcept;

        /* Convenience operators */
        inline const Type& operator[](TypeId typeId) const
        {
            return get(typeId);
        }

        /* Convenience operators */
        inline TypeId operator[](Type &&type)
        {
            return getOrInsert(std::forward<Type>(type));
        }

        static HYCORE_API size_t getTypeHash(const Type &type);

    private:
        struct Impl;
        std::unique_ptr<Impl> impl_;
    };
}

namespace hycore::constants
{
    struct IntegerC
    {
        hycore::types::IntegerT type;
        std::vector<std::uint8_t> value; // Little-endian byte representation
    };

    HYCORE_API IntegerC i1c(bool val);
    HYCORE_API IntegerC i8c(uint8_t val);
    HYCORE_API IntegerC i16c(uint16_t val);
    HYCORE_API IntegerC i32c(uint32_t val);
    HYCORE_API IntegerC i64c(uint64_t val);

    struct FloatingPointC
    {
        hycore::types::FloatingPointT type;
        double value;
    };

    using VectorC = std::variant<std::vector<IntegerC>, std::vector<FloatingPointC>>;

    struct StructC
    {
        std::vector<std::variant<IntegerC, FloatingPointC, VectorC>> memberValues;
    };

    using Constant = std::variant<IntegerC, FloatingPointC, VectorC, StructC>;
}

namespace std
{
    template <>
    struct hash<hycore::types::VoidT>
    {
        std::size_t operator()(const hycore::types::VoidT &) const noexcept
        {
            return 0xba92e13a;
        }
    };

    template <>
    struct hash<hycore::types::LabelT>
    {
        std::size_t operator()(const hycore::types::LabelT &) const noexcept
        {
            return 0x538587d9;
        }
    };

    template <>
    struct hash<hycore::types::IntegerT>
    {
        std::size_t operator()(const hycore::types::IntegerT &type) const
        {
            size_t h1 = std::hash<std::uint16_t>()(type.bitWidth);
            return 0x413f13c0 ^ (h1 << 1);
        }
    };

    template <>
    struct hash<hycore::types::FloatingPointT>
    {
        std::size_t operator()(const hycore::types::FloatingPointT &type) const
        {
            size_t h1 = std::hash<int>()(static_cast<int>(type));
            return 0x7f4a7c15 ^ (h1 << 1);
        }
    };

    template <>
    struct hash<hycore::types::PointerT>
    {
        std::size_t operator()(const hycore::types::PointerT &type) const
        {
            size_t h1 = std::hash<hycore::TypeId>()(type.pointeeType);
            return 0x7cb171cc ^ (h1 << 1);
        }
    };

    template <>
    struct hash<hycore::types::VectorT>
    {
        std::size_t operator()(const hycore::types::VectorT &type) const
        {
            size_t h1 = std::hash<hycore::TypeId>()(type.elementType);
            size_t h2 = std::hash<std::uint32_t>()(type.elementCount);
            size_t h3 = std::hash<bool>()(type.isScalable);
            return 0x3a847025 ^ (h1 << 1) ^ (h2 << 2) ^ (h3 << 3);
        }
    };

    template <>
    struct hash<hycore::types::FunctionT>
    {
        std::size_t operator()(const hycore::types::FunctionT &type) const
        {
            size_t h1 = std::hash<hycore::TypeId>()(type.returnType);
            size_t h2 = 0;
            for (const auto &paramType : type.parameterTypes)
            {
                h2 ^= std::hash<hycore::TypeId>()(paramType) + 0x895339da + (h2 << 6) + (h2 >> 2);
            }
            return 0x895339da ^ (h1 << 1) ^ (h2 << 2);
        }
    };

    template <>
    struct hash<hycore::types::StructT>
    {
        std::size_t operator()(const hycore::types::StructT &type) const
        {
            size_t h = 0;
            for (const auto &memberType : type.memberTypes)
            {
                h ^= std::hash<hycore::TypeId>()(memberType) + 0x5d1e1198 + (h << 6) + (h >> 2);
            }
            return 0xa49d1b63 ^ (h << 1);
        }
    };

    template <>
    struct hash<hycore::types::Type>
    {
        std::size_t operator()(const hycore::types::Type &type) const
        {
            return std::visit([](const auto &t)
                              { return std::hash<std::decay_t<decltype(t)>>()(t); }, type);
        }
    };
}
