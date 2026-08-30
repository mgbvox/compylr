# typescript-go-bridge Specification

## Purpose
The host bridge for the `(typescript, go)` pair: it generates what is needed to build, wrap, and
invoke compiled Go from a TypeScript or JavaScript runtime. Being keyed by the pair rather than by
the target is the point — sharing a backend with another pair would not mean sharing a calling
convention.

## Requirements

### Requirement: Implement the host bridge for TypeScript and Go
The `compylr-bridge-typescript-golang` crate SHALL implement the host bridge interface, reporting
`typescript` as its source and `go` as its target. It SHALL emit its layer as text and SHALL NOT
itself link the runtime it targets.

#### Scenario: The pair resolves to this bridge
- **GIVEN** a registry with the implemented bridges
- **WHEN** the bridge for the `(typescript, go)` pair is looked up
- **THEN** this bridge is returned

#### Scenario: The bridge does not link what it generates
- **GIVEN** the workspace manifests
- **WHEN** the crate boundaries are checked
- **THEN** this crate depends on no Node or CGo runtime

### Requirement: Emit a complete host artifact
The bridge SHALL emit a host artifact carrying the generated Go source, the C-shared export
wrappers that make it callable, a TypeScript declaration file, and a loader that configures the
runtime binding.

#### Scenario: The artifact carries everything needed to build and call
- **GIVEN** a validated unit and a build key
- **WHEN** the bridge emits for that unit
- **THEN** the artifact carries the module definition, the generated Go, the runtime shim, the
  export wrappers, the loader, and the declaration file

### Requirement: Boundary marshalling and error propagation
The bridge SHALL generate marshalling for values crossing the runtime boundary: numbers, booleans,
and text as primitives, and sequences, mappings, and instances as structured values. A failure
reported by the generated Go SHALL surface in the host as a thrown error carrying the failure's
message.

#### Scenario: A value survives the round trip
- **GIVEN** a compiled unit loaded into a JavaScript runtime
- **WHEN** a compiled function is called with arguments from the host
- **THEN** the arguments cross as the types the unit declared
- **AND** the answer returns as the host's own type for that IR type

#### Scenario: A reported failure becomes a thrown error
- **GIVEN** a compiled unit whose behavior reports division by zero
- **WHEN** a compiled function divides by zero
- **THEN** the host throws
- **AND** the thrown error carries the failure's message
- **BUT** the process does not abort

### Requirement: Loadable name uniqueness
The bridge SHALL encode both the unit's fingerprint and the build key's variant tag into the name
the artifact loads under, so that two builds of the same program under different configurations do
not collide in a runtime's module cache.

#### Scenario: Two builds of one program load as distinct modules
- **GIVEN** one program emitted twice under different build keys
- **WHEN** the loadable names are compared
- **THEN** they differ
- **AND** each carries the fingerprint and the variant tag it was built under
