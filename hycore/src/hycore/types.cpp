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
