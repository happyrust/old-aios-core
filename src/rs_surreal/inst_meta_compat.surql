DEFINE FUNCTION OVERWRITE fn::refno_u64($r: record) {
    LET $raw = record::id($r);
    LET $id = if type::is::array($raw) { <string> array::at($raw, 0) } else { <string> $raw };
    LET $parts = string::split($id, '_');
    RETURN <int> $parts[0] * 4294967296 + <int> $parts[1];
};

DEFINE FUNCTION OVERWRITE fn::anc_u64($r: record) {
    LET $chain = if $r.id == none { [] } else {
        array::complement(
            (SELECT VALUE [id,
                owner,
                owner.owner,
                owner.owner.owner,
                owner.owner.owner.owner,
                owner.owner.owner.owner.owner,
                owner.owner.owner.owner.owner.owner,
                owner.owner.owner.owner.owner.owner.owner,
                owner.owner.owner.owner.owner.owner.owner.owner,
                owner.owner.owner.owner.owner.owner.owner.owner.owner,
                owner.owner.owner.owner.owner.owner.owner.owner.owner.owner,
                owner.owner.owner.owner.owner.owner.owner.owner.owner.owner.owner,
                owner.owner.owner.owner.owner.owner.owner.owner.owner.owner.owner.owner]
             FROM ONLY $r LIMIT 1),
            [NONE, pe:0_0]
        )
    };
    RETURN array::map($chain, |$a| fn::refno_u64($a));
};
