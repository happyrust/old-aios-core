use rkyv::{Serialize, ser::serializers::AllocSerializer};

pub fn gen_bytes_hash<T, const N: usize>(v: &T) -> u64
where
    T: Serialize<AllocSerializer<N>>
        + rkyv::Serialize<
            rkyv::ser::serializers::CompositeSerializer<
                rkyv::ser::serializers::AlignedSerializer<rkyv::AlignedVec>,
                rkyv::ser::serializers::FallbackScratch<
                    rkyv::ser::serializers::HeapScratch<512>,
                    rkyv::ser::serializers::AllocScratch,
                >,
                rkyv::ser::serializers::SharedSerializeMap,
            >,
        >,
{
    use core::hash::Hasher;

    let bytes = rkyv::to_bytes::<T, N>(v).unwrap().to_vec();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash_slice(&bytes, &mut hasher);
    hasher.finish()
}
