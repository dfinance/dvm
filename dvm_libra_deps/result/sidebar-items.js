initSidebarItems({"enum":[["AbortLoc","An `AbortLocation` specifies where a Move program `abort` occurred, either in a function in a module, or in a script"],["KeptVMStatus",""],["Location",""],["StatusCode","We don’t derive Arbitrary on this enum because it is too large and breaks proptest. It is written for a subset of these in proptest_types. We test conversion between this and protobuf with a hand-written test."],["VMStatus","A `VMStatus` is represented as either"]],"struct":[["PartialVMError",""],["VMError",""]],"type":[["DiscardedVMStatus",""],["PartialVMResult",""],["VMResult",""]]});