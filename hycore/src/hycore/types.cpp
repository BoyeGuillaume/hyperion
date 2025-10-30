#include <hycore/types.hpp>
#include <vector>
#include <tuple>
#include <algorithm>

namespace hycore::types
{

struct TypeRegistry::Impl
{
    std::vector<Type> types;
    std::vector<std::tuple<size_t /* hash */, TypeId /* typeid */>> typeCache;

    inline void insertType(auto it, std::tuple<size_t, TypeId> typeEntry) {
        typeCache.insert(it, typeEntry);
    }
};

HYCORE_API TypeRegistry::TypeRegistry()
: impl_(std::make_unique<Impl>())
{
}

HYCORE_API TypeRegistry::~TypeRegistry()
{
}

HYCORE_API const Type& TypeRegistry::get(TypeId typeId) const
{
    if (typeId >= impl_->types.size())
    {
        throw std::out_of_range("TypeId out of range");
    }
    return impl_->types[typeId];
}

HYCORE_API TypeId TypeRegistry::getOrInsert(Type &&type)
{
    size_t typeHash = getTypeHash(type);

    // Binary search through the cache
    auto &cache = impl_->typeCache;
    auto it = std::lower_bound(cache.begin(), cache.end(), typeHash,
        [](const auto &lhs, const auto &rhsHash)
        {
            return std::get<0>(lhs) < rhsHash;
        });

    // Check for existing types with the same hash
    while (it != cache.end() && std::get<0>(*it) == typeHash)
    {
        TypeId existingTypeId = std::get<1>(*it);
        if (impl_->types[existingTypeId] == type)
        {
            return existingTypeId;
        }
        ++it;
    }

    // Type not found, insert new type
    TypeId newTypeId = static_cast<TypeId>(impl_->types.size());
    impl_->types.push_back(type);
    impl_->insertType(it, std::make_tuple(typeHash, newTypeId));
    return newTypeId;
}

HYCORE_API size_t TypeRegistry::getTypeHash(const Type &type)
{
    return std::hash<Type>{}(type);
}

HYCORE_API size_t TypeRegistry::size() const noexcept
{
    return impl_->types.size();
}

}

namespace hycore::constants
{
    HYCORE_API IntegerC i1c(bool val) {
        IntegerC constant;
        constant.type.bitWidth = 1;
        constant.value.resize(1);
        constant.value[0] = val ? 0xff : 0;
        return constant;
    }

    HYCORE_API IntegerC i8c(uint8_t val) {
        IntegerC constant;
        constant.type.bitWidth = 8;
        constant.value.resize(1);
        constant.value[0] = val;
        return constant;
    }

    HYCORE_API IntegerC i16c(uint16_t val) {
        IntegerC constant;
        constant.type.bitWidth = 16;
        constant.value.resize(2);
        constant.value[0] = static_cast<std::uint8_t>(val & 0xFF);
        constant.value[1] = static_cast<std::uint8_t>((val >> 8) & 0xFF);
        return constant;
    }

    HYCORE_API IntegerC i32c(uint32_t val) {
        IntegerC constant;
        constant.type.bitWidth = 32;
        constant.value.resize(4);
        for (size_t i = 0; i < 4; ++i) {
            constant.value[i] = static_cast<std::uint8_t>((val >> (i * 8)) & 0xFF);
        }
        return constant;
    }

    HYCORE_API IntegerC i64c(uint64_t val) {
        IntegerC constant;
        constant.type.bitWidth = 64;
        constant.value.resize(8);
        for (size_t i = 0; i < 8; ++i) {
            constant.value[i] = static_cast<std::uint8_t>((val >> (i * 8)) & 0xFF);
        }
        return constant;
    }
}