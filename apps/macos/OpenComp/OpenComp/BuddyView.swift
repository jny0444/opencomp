import SwiftUI

struct StepLine: Identifiable {
    let id = UUID()
    var text: String
}

struct BuddyView: View {
    @State private var task = ""
    @State private var steps: [StepLine] = []
    @State private var running = false
    @State private var job: Task<Void, Never>?

    private var trimmedTask: String {
        task.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var status: String {
        if running {
            return "Running"
        }
        if steps.isEmpty {
            return "Idle"
        }
        return "Finished"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            Divider()
            controls
            Divider()
            log
        }
        .frame(width: 340, height: 420)
    }

    private var header: some View {
        HStack(spacing: 10) {
            Image(systemName: "desktopcomputer")
                .font(.title2)
                .foregroundStyle(.secondary)
            VStack(alignment: .leading, spacing: 2) {
                Text("OpenComp")
                    .font(.headline)
                Text(status)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if running {
                ProgressView()
                    .controlSize(.small)
            }
        }
        .padding(16)
    }

    private var controls: some View {
        VStack(alignment: .leading, spacing: 10) {
            TextField("What should this Mac do?", text: $task)
                .textFieldStyle(.roundedBorder)
                .onSubmit(start)
                .disabled(running)

            HStack {
                Button("Run", action: start)
                    .keyboardShortcut(.defaultAction)
                    .disabled(running || trimmedTask.isEmpty)
                Button("Stop", action: stop)
                    .disabled(!running)
            }
        }
        .padding(16)
    }

    private var log: some View {
        Group {
            if steps.isEmpty {
                Text("Steps show up here.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    .padding(16)
            } else {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 8) {
                        ForEach(steps) { step in
                            Text(step.text)
                                .font(.callout)
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }
                    .padding(16)
                }
            }
        }
    }

    private func start() {
        guard !running, !trimmedTask.isEmpty else { return }
        job?.cancel()
        steps = []
        running = true
        let current = trimmedTask
        job = Task {
            for number in 1...3 {
                try? await Task.sleep(for: .seconds(1))
                if Task.isCancelled { return }
                steps.append(StepLine(text: "Step \(number) · \(current)"))
            }
            steps.append(StepLine(text: "demo finished"))
            running = false
        }
    }

    private func stop() {
        job?.cancel()
        job = nil
        if running {
            steps.append(StepLine(text: "Stopped"))
        }
        running = false
    }
}

#Preview {
    BuddyView()
}
