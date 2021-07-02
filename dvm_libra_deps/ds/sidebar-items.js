initSidebarItems({"enum":[["TypeTag",""],["WriteOp",""]],"struct":[["AccessPath",""],["ModuleCache",""],["RemoteStorage",""],["ResourceKey","Represents the intitial key into global storage where we first index by the address, and then the struct tag"],["ScriptCache",""],["StructTag",""],["TransactionDataCache","Transaction data cache. Keep updates within a transaction so they can all be published at once when the transaction succeeeds."],["TypeCache",""],["WriteSet","`WriteSet` contains all access paths that one transaction modifies. Each of them is a `WriteOp` where `Value(val)` means that serialized representation should be updated to `val`, and `Deletion` means that we are going to delete this access path."],["WriteSetMut","A mutable version of `WriteSet`."]],"trait":[["DataStore","Provide an implementation for bytecodes related to data with a given data store."],["RemoteCache","Trait for the Move VM to abstract storage operations."]]});