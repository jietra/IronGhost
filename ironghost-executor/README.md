# IronGhost Executor

IronGhost Executor is the execution layer of the IronGhost ecosystem.

Its responsibility is to execute artifacts produced by agents in a controlled environment and return runtime feedback.

## Architecture

```text
ExecutionRequest
        ↓
ExecutorServer
        ↓
ExecutorOrchestrator
        ↓
EnvironmentManager
        ↓
Environment
        ↓
RuntimeExecutor
        ↓
ExecutionVerdict
```

The system intentionally separates:

- **Environment**: where execution happens (Local, Kali, etc.)
- **Runtime**: how execution happens (Python, Bash, PowerShell, etc.)

This allows the same runtime executor to operate across multiple environments.

For example:

```text
PythonExecutor
    ├── LocalEnvironment
    └── KaliEnvironment
```

without changing any runtime logic.

## Available Environments

### Local

Local execution environment.

⚠️ This environment is intended for development and testing only.

It executes code directly on the host machine and provides no isolation.

### Kali

Kali Linux execution environment.

Currently the VM must be provisioned manually.

Requirements:

- Kali Linux VM
- SSH enabled
- Key-based authentication configured
- SSH alias:

```bash
ssh ironghost-worker
```

The executor uses SSH and SCP to upload artifacts and execute commands remotely.

Automatic environment provisioning will be implemented in future versions.

## Available Runtimes

### Python

Executes Python scripts inside the selected environment.

The same Python runtime can be used on:

- Local
- Kali
- future environments (Ubuntu, Windows, AD labs, etc.)

## Running

```bash
cargo run
```

The executor listens on:

```text
/tmp/executor.sock
```

## Test with socat

```bash
echo '{ \
    "environment":"kali",   \
    "runtime":"python",     \
    "source":"print(\"Hello IronGhost\")"   \
}' | socat - UNIX-CONNECT:/tmp/executor.sock
```

Example response:

```json
{
  "success": true,
  "stdout": "Hello IronGhost\n",
  "stderr": "",
  "exit_code": 0
}
```

## Security Notice

The current implementation is an execution prototype.

It is NOT intended to execute untrusted code in production.

Future versions will introduce:

- VM provisioning
- snapshot management
- disposable environments
- stronger isolation
- micro-VM support

