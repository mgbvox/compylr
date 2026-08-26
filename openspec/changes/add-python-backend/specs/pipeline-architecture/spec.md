## ADDED Requirements

### Requirement: A backend names its own translated-source file

A backend that emits several files SHALL declare which of them holds the translated functions.

A caller wanting only the translation SHALL obtain that file through the declaration, and SHALL NOT
identify it by knowing which backend it selected. Naming one backend's file to serve a request that
any backend can satisfy is how a pipeline acquires a default target without saying so, and it fails
the moment a second backend is added.

#### Scenario: The translated file is obtained by asking

- **WHEN** a caller requests only the translated source for a selected backend
- **THEN** it obtains that backend's translated-source file through the backend's own declaration

#### Scenario: Any implemented backend answers

- **WHEN** the translated source is requested for each implemented backend in turn
- **THEN** each request succeeds, and no backend is identified by name to serve it
