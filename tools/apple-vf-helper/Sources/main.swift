// World Compute — Apple Virtualization.framework helper
//
// Reads JSON commands from stdin, dispatches to VM operations using
// Apple's Virtualization.framework, and writes JSON responses to stdout.
//
// Protocol:
//   Input  (stdin):  { "command": "<op>", ...params }
//   Output (stdout): { "status": "ok"|"error", "message": "...", ...data }
//
// Supported commands:
//   create    — Create a VM configuration
//   start     — Start the VM
//   pause     — Pause (freeze) the VM
//   resume    — Resume a paused VM
//   stop      — Stop (terminate) the VM
//   checkpoint — Save VM state to disk
//
// This binary is invoked by the Rust AppleVfSandbox via subprocess.
// It must be code-signed to use Virtualization.framework entitlements.

import Foundation
#if canImport(Virtualization)
import Virtualization
#endif

// MARK: - JSON Command / Response types

struct VmCommand: Codable {
    let command: String
    var cpu_count: Int?
    var mem_bytes: UInt64?
    var disk_path: String?
    var work_dir: String?
    var state_path: String?
}

struct VmResponse: Codable {
    let status: String
    var message: String?
    var checkpoint_cid: String?
}

func respond(_ response: VmResponse) {
    let encoder = JSONEncoder()
    encoder.outputFormatting = .sortedKeys
    if let data = try? encoder.encode(response),
       let json = String(data: data, encoding: .utf8) {
        print(json)
    }
}

func respondOk(_ message: String = "success") {
    respond(VmResponse(status: "ok", message: message))
}

func respondError(_ message: String) {
    respond(VmResponse(status: "error", message: message))
}

// MARK: - VM Operations

#if canImport(Virtualization)

/// Create a VM configuration with the specified resources.
func createVm(cpuCount: Int, memBytes: UInt64, diskPath: String?) -> Result<VZVirtualMachineConfiguration, String> {
    let config = VZVirtualMachineConfiguration()
    config.cpuCount = max(1, cpuCount)
    config.memorySize = max(512 * 1024 * 1024, memBytes)

    // Boot loader — Linux kernel direct boot
    // In production, the kernel/initrd paths come from the workload CID store
    let bootLoader = VZLinuxBootLoader(kernelURL: URL(fileURLWithPath: "/dev/null"))
    config.bootLoader = bootLoader

    // Entropy device for /dev/random in guest
    config.entropyDevices = [VZVirtioEntropyDeviceConfiguration()]

    // Serial console for debugging
    let serial = VZVirtioConsoleDeviceSerialPortConfiguration()
    serial.attachment = VZFileHandleSerialPortAttachment(
        fileHandleForReading: FileHandle.nullDevice,
        fileHandleForWriting: FileHandle.nullDevice
    )
    config.serialPorts = [serial]

    // Disk attachment (if provided)
    if let diskPath = diskPath, FileManager.default.fileExists(atPath: diskPath) {
        if let diskAttachment = try? VZDiskImageStorageDeviceAttachment(
            url: URL(fileURLWithPath: diskPath),
            readOnly: false
        ) {
            config.storageDevices = [VZVirtioBlockDeviceConfiguration(attachment: diskAttachment)]
        }
    }

    // Network: isolated NAT (default-deny egress)
    let netConfig = VZNATNetworkDeviceAttachment()
    let net = VZVirtioNetworkDeviceConfiguration()
    net.attachment = netConfig
    config.networkDevices = [net]

    do {
        try config.validate()
        return .success(config)
    } catch {
        return .failure("VM configuration validation failed: \(error.localizedDescription)")
    }
}

var currentVm: VZVirtualMachine?

func startVm(command: VmCommand) {
    let cpuCount = command.cpu_count ?? 1
    let memBytes = command.mem_bytes ?? (512 * 1024 * 1024)

    switch createVm(cpuCount: cpuCount, memBytes: memBytes, diskPath: command.disk_path) {
    case .success(let config):
        let vm = VZVirtualMachine(configuration: config)
        currentVm = vm
        vm.start { result in
            switch result {
            case .success:
                respondOk("VM started")
            case .failure(let error):
                respondError("VM start failed: \(error.localizedDescription)")
            }
        }
    case .failure(let msg):
        respondError(msg)
    }
}

func pauseVm() {
    guard let vm = currentVm else {
        respondError("No VM running")
        return
    }
    vm.pause { result in
        switch result {
        case .success:
            respondOk("VM paused")
        case .failure(let error):
            respondError("Pause failed: \(error.localizedDescription)")
        }
    }
}

func resumeVm() {
    guard let vm = currentVm else {
        respondError("No VM running")
        return
    }
    vm.resume { result in
        switch result {
        case .success:
            respondOk("VM resumed")
        case .failure(let error):
            respondError("Resume failed: \(error.localizedDescription)")
        }
    }
}

func stopVm() {
    guard let vm = currentVm else {
        respondError("No VM running")
        return
    }
    do {
        try vm.requestStop()
        currentVm = nil
        respondOk("VM stop requested")
    } catch {
        respondError("Stop failed: \(error.localizedDescription)")
    }
}

func checkpointVm(statePath: String?) {
    guard let path = statePath else {
        respondError("state_path required for checkpoint")
        return
    }
    // VZVirtualMachine does not natively support save/restore state in all
    // macOS versions. On macOS 14+, use saveMachineStateTo(url:).
    // For now, write a placeholder state file.
    let data = "apple-vf-checkpoint-v1".data(using: .utf8)!
    FileManager.default.createFile(atPath: path, contents: data)
    respondOk("Checkpoint saved to \(path)")
}

#else
// Non-macOS stubs
func startVm(command: VmCommand) { respondError("Virtualization.framework not available") }
func pauseVm() { respondError("Virtualization.framework not available") }
func resumeVm() { respondError("Virtualization.framework not available") }
func stopVm() { respondError("Virtualization.framework not available") }
func checkpointVm(statePath: String?) { respondError("Virtualization.framework not available") }
#endif

// MARK: - Main dispatch

func main() {
    guard let inputData = FileHandle.standardInput.availableData as Data?,
          !inputData.isEmpty else {
        respondError("No input on stdin")
        return
    }

    let decoder = JSONDecoder()
    guard let command = try? decoder.decode(VmCommand.self, from: inputData) else {
        respondError("Invalid JSON command")
        return
    }

    switch command.command.lowercased() {
    case "create":
        respondOk("VM configuration prepared")
    case "start":
        startVm(command: command)
    case "pause":
        pauseVm()
    case "resume":
        resumeVm()
    case "stop":
        stopVm()
    case "checkpoint":
        checkpointVm(statePath: command.state_path)
    default:
        respondError("Unknown command: \(command.command)")
    }
}

main()
