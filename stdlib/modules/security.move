address 0x1 {

/// Security is a new pattern-candidate for standard library.
/// In basic terms it's locker-key pair, where locker may contain any type
/// of asset in it, while key is a hot potato which cannot be stored directly
/// at any address and must be placed somewhere else.
///
/// Owner of the locker (or Security) knows what's inside, he can read the stored
/// data and estimate its value (or know something about the deal/reasons for
/// creation) while not being able to unlock this locker without the proper
/// key for it. Key may be placed into a Financial contract module which will
/// control the conditions under which this locker can be unlocked (in terms
/// of Move - destroyed). While coupon itself may travel through chain carrying
/// the data hidden in it.
///
/// In terms of security this pair is safe, as coupon-locker and its destroy
/// permission-key are matched by signer's address and ID. For every coupon
/// there's only one key (permission) which can unlock it.
///
/// It is also possible to let governance (0x1 or different address) unlock
/// these coupons if key got lost or deal is no longer profitable. Though it is
/// up to creator of the coupon to decide what should be put inside.
///
/// Multiple coupons might be united by the same dataset stored inside of them.
/// This can allow developers implement their multi-coupon sets. And only
/// imagination is the limit to these features.
///
/// I also mean the coupon as some analogy to financial security. Creator of the
/// coupon is the Issuer of the security, he knows what this security means and
/// he defines the way of how it can be used. And while in this case the coupon
/// would carry only some information about an underlying asset we prevent this
/// asset from moving across the network - hence we bypass the obligation to track
/// every asset in chain.
module Security {

    use 0x1::Time;
    use 0x1::Signer;

    const ERR_UNABLE_TO_PROVE : u64 = 101;
    const ERR_NOT_EXPIRED : u64 = 102;
    const ERR_WRONG_EXPIRATION : u64 = 103;

    resource struct Info {
        securities_count: u64
    }

    /// Security itself. Should be used to hold descriptors
    /// and information to find Proof. Copyable is put
    /// intentionally to avoid cases where assets are locked
    /// in forever-living securitites with no access to Proof.
    resource struct Security<For: copyable> {
        for: For,
        by: address,
        id: u64,
        exp: u64
    }

    /// Proof resource which 'proves' Security. When these two
    /// meet they can be destroyed therefore proving that
    /// Security matches its Proof and can be used to get access
    /// to assets or business logic.
    resource struct Proof {
        by: address,
        id: u64,
        exp: u64
    }

    /// Issue security with expiration date.
    /// Expiration can be set to 0, then it will never expire.
    public fun issue<For: copyable>(
        account: &signer,
        for: For,
        exp: u64
    ): (Security<For>, Proof) acquires Info {

        if (exp != 0) {
            assert(exp > Time::now(), ERR_WRONG_EXPIRATION);
        };

        let by = Signer::address_of(account);
        let id = if (!has_info(by)) {
            move_to(account, Info { securities_count: 1 });
            1
        } else {
            let info = borrow_global_mut<Info>(by);
            info.securities_count = info.securities_count + 1;
            info.securities_count
        };

        (
            Security { for, by, id, exp },
            Proof { by, id, exp }
        )
    }

    /// Issue Security with no expiration date
    public fun issue_forever<For: copyable>(
        account: &signer,
        for: For
    ): (Security<For>, Proof) acquires Info {
        issue<For>(account, for, 0)
    }

    /// Check whether user already has securities issued from his account
    public fun has_info(account: address): bool {
        exists<Info>(account)
    }

    /// Read security details - issuer, ID and expiration date
    public fun read_security<For: copyable>(
        security: &Security<For>
    ): (address, u64, u64) {
        (
            security.by,
            security.id,
            security.exp
        )
    }

    /// Get immutable reference to Security contents/details
    public fun borrow<For: copyable>(security: &Security<For>): &For {
        &security.for
    }

    /// Safe check whether Security can be proved with Proof
    public fun can_prove<For: copyable>(security: &Security<For>, proof: &Proof): bool {
        (security.id == proof.id && security.by == proof.by)
    }

    /// Prove that Security matches given Proof. This method makes sure that
    /// issuer account address and security ID match. When success - releases
    /// data (or resource) stored inside Security.
    public fun prove<For: copyable>(
        security: Security<For>,
        proof: Proof
    ): For {

        assert(security.by == proof.by, ERR_UNABLE_TO_PROVE);
        assert(security.id == proof.id, ERR_UNABLE_TO_PROVE);

        let Security { by: _, id: _, exp: _, for } = security;
        let Proof { by: _, id: _, exp: _ } = proof;

        for
    }

    /// Destroy Security if it has expired (if expiration was set i.e. non-zero)
    public fun destroy_expired_security<For: copyable>(security: Security<For>): For {
        assert(security.exp != 0, ERR_NOT_EXPIRED);
        assert(security.exp <= Time::now(), ERR_NOT_EXPIRED);

        let Security { exp: _, id: _, by: _, for } = security;

        for
    }

    /// Destroy Proof if it has expired
    public fun destroy_expired_proof(proof: Proof) {
        assert(proof.exp != 0, ERR_NOT_EXPIRED);
        assert(proof.exp <= Time::now(), ERR_NOT_EXPIRED);

        let Proof { exp: _, id: _, by: _ } =  proof;
    }
}

module SecurityStorage {

    use 0x1::Security::{Security};
    use 0x1::Vector;
    use 0x1::Signer;

    resource struct T<For: copyable> {
        securities: vector<Security<For>>
    }

    public fun init<For: copyable>(account: &signer) {
        move_to<T<For>>(account, T {
            securities: Vector::empty<Security<For>>()
        });
    }

    public fun push<For: copyable>(
        account: &signer,
        security: Security<For>
    ) acquires T {
        Vector::push_back(
            &mut borrow_global_mut<T<For>>(Signer::address_of(account)).securities
            , security
        );
    }

    public fun take<For: copyable>(
        account: &signer,
        el: u64
    ): Security<For> acquires T {
        let me  = Signer::address_of(account);
        let vec = &mut borrow_global_mut<T<For>>(me).securities;

        Vector::remove(vec, el)
    }
}
}
