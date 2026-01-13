// StatisticsViews - Element statistics and distribution views
// Contains confidence and label distribution visualizations

import SwiftUI
import DoclingBridge

// MARK: - Element Statistics View

struct ElementStatisticsView: View {
    let elements: [Element]

    var body: some View {
        if elements.isEmpty {
            Text("No elements detected")
                .foregroundColor(.secondary)
                .font(.caption)
        } else {
            VStack(alignment: .leading, spacing: 12) {
                // Confidence distribution
                ConfidenceDistributionView(elements: elements)

                Divider()

                // Label distribution (top 5)
                LabelDistributionView(elements: elements)
            }
        }
    }
}

// MARK: - Confidence Distribution View

struct ConfidenceDistributionView: View {
    let elements: [Element]

    var lowCount: Int {
        elements.filter { $0.confidence < 0.5 }.count
    }

    var mediumCount: Int {
        elements.filter { $0.confidence >= 0.5 && $0.confidence < 0.8 }.count
    }

    var highCount: Int {
        elements.filter { $0.confidence >= 0.8 }.count
    }

    var averageConfidence: Float {
        guard !elements.isEmpty else { return 0 }
        return elements.map { $0.confidence }.reduce(0, +) / Float(elements.count)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text("Confidence")
                    .font(.caption.bold())
                Spacer()
                Text(String(format: "Avg: %.0f%%", averageConfidence * 100))
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            // Mini bar chart
            HStack(spacing: 2) {
                ConfidenceBar(count: highCount, total: elements.count, color: .green, label: "High")
                ConfidenceBar(count: mediumCount, total: elements.count, color: .orange, label: "Med")
                ConfidenceBar(count: lowCount, total: elements.count, color: .red, label: "Low")
            }
            .frame(height: 24)

            // Legend
            HStack(spacing: 12) {
                LegendItem(color: .green, label: "≥80%", count: highCount)
                LegendItem(color: .orange, label: "50-80%", count: mediumCount)
                LegendItem(color: .red, label: "<50%", count: lowCount)
            }
            .font(.caption2)
        }
    }
}

// MARK: - Confidence Bar

struct ConfidenceBar: View {
    let count: Int
    let total: Int
    let color: Color
    let label: String

    var percentage: Double {
        guard total > 0 else { return 0 }
        return Double(count) / Double(total)
    }

    var body: some View {
        GeometryReader { geo in
            RoundedRectangle(cornerRadius: 2)
                .fill(color.opacity(count > 0 ? 0.8 : 0.1))
                .frame(width: max(geo.size.width * percentage, count > 0 ? 4 : 0))
        }
    }
}

// MARK: - Legend Item

struct LegendItem: View {
    let color: Color
    let label: String
    let count: Int

    var body: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(color)
                .frame(width: 6, height: 6)
            Text("\(label): \(count)")
                .foregroundColor(.secondary)
        }
    }
}

// MARK: - Label Distribution View

struct LabelDistributionView: View {
    let elements: [Element]

    /// Sorted label counts (top 5)
    var labelCounts: [(label: DocItemLabel, count: Int)] {
        var counts: [DocItemLabel: Int] = [:]
        for element in elements {
            counts[element.label, default: 0] += 1
        }
        let sorted = counts.map { (label: $0.key, count: $0.value) }
            .sorted(by: { $0.count > $1.count })
        return Array(sorted.prefix(5))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Top Labels")
                .font(.caption.bold())

            ForEach(labelCounts, id: \.label) { item in
                HStack(spacing: 6) {
                    Circle()
                        .fill(item.label.swiftUIColor)
                        .frame(width: 8, height: 8)
                    Text(item.label.shortDescription)
                        .font(.caption)
                    Spacer()
                    Text("\(item.count)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    // Mini bar
                    ProgressView(value: Double(item.count), total: Double(elements.count))
                        .frame(width: 40)
                }
            }
        }
    }
}

// MARK: - Label Filter View

struct LabelFilterView: View {
    @ObservedObject var viewModel: DocumentViewModel

    /// Labels that have elements in current snapshot
    var availableLabels: [DocItemLabel] {
        guard let snapshot = viewModel.currentStageSnapshot else { return [] }
        let labelSet = Set(snapshot.elements.map { $0.label })
        return DocItemLabel.allCases.filter { labelSet.contains($0) }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Quick actions
            HStack {
                Button("All") {
                    viewModel.clearLabelFilter()
                }
                .buttonStyle(.bordered)
                .controlSize(.small)

                Button("None") {
                    viewModel.selectedLabels = Set(availableLabels)
                    // Invert: if all selected, we want to hide all
                    viewModel.selectedLabels = []
                    for label in availableLabels {
                        viewModel.selectedLabels.insert(label)
                    }
                    // Clear to show none by selecting all then toggling logic
                    // Actually simpler: set selectedLabels to empty set that matches nothing
                    // But our logic is: empty = show all. So to hide all, we need
                    // a non-empty set that doesn't match any present labels.
                    // Simplest: create a set with a label not in availableLabels
                    // Actually, let's just make None select all available, since
                    // our filter logic is: if selectedLabels is not empty, only show those
                    // So to hide all, we need selectedLabels to contain labels not in snapshot
                    // This is confusing. Let's change approach:
                    // "None" should result in no visible elements
                    // Our filter: empty = show all, non-empty = show only those in set
                    // So "None" should set selectedLabels to a label that doesn't exist
                    // Actually, let's just skip "None" - it's not useful
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
                .hidden()  // Hide for now, logic is confusing

                Spacer()
            }

            // Label toggles in 2-column grid
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 4) {
                ForEach(availableLabels) { label in
                    LabelToggleButton(
                        label: label,
                        isSelected: viewModel.isLabelVisible(label),
                        count: countForLabel(label),
                        action: {
                            if viewModel.selectedLabels.isEmpty {
                                // If showing all, clicking one shows only that one
                                viewModel.showOnlyLabel(label)
                            } else if viewModel.selectedLabels.count == 1 && viewModel.selectedLabels.contains(label) {
                                // If only this label is shown, clear filter to show all
                                viewModel.clearLabelFilter()
                            } else {
                                // Toggle this label
                                viewModel.toggleLabel(label)
                            }
                        }
                    )
                }
            }

            if !viewModel.selectedLabels.isEmpty {
                Button("Clear Filter") {
                    viewModel.clearLabelFilter()
                }
                .buttonStyle(.link)
                .font(.caption)
            }
        }
    }

    func countForLabel(_ label: DocItemLabel) -> Int {
        guard let snapshot = viewModel.currentStageSnapshot else { return 0 }
        return snapshot.elements.filter { $0.label == label }.count
    }
}

// MARK: - Label Toggle Button

struct LabelToggleButton: View {
    let label: DocItemLabel
    let isSelected: Bool
    let count: Int
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 4) {
                Circle()
                    .fill(label.swiftUIColor)
                    .frame(width: 8, height: 8)
                Text(label.shortDescription)
                    .font(.caption2)
                    .lineLimit(1)
                Spacer()
                Text("\(count)")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
            .padding(.horizontal, 6)
            .padding(.vertical, 4)
            .background(isSelected ? label.swiftUIColor.opacity(0.2) : Color.clear)
            .cornerRadius(4)
            .overlay(
                RoundedRectangle(cornerRadius: 4)
                    .stroke(isSelected ? label.swiftUIColor : Color.secondary.opacity(0.3), lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Validation Issue Types

/// Types of annotation validation issues
public enum ValidationIssue: Identifiable, Hashable {
    case tinyBox(elementId: UInt32)           // Box too small (< 10 pt)
    case outOfBounds(elementId: UInt32)       // Extends beyond page
    case lowConfidence(elementId: UInt32)     // Confidence < 50%
    case extremeAspectRatio(elementId: UInt32) // Very thin/wide
    case significantOverlap(element1: UInt32, element2: UInt32) // > 50% overlap
    case duplicateReadingOrder(order: Int32, elements: [UInt32]) // Same reading order
    case readingOrderGap(expectedOrder: Int32)  // Missing number in sequence
    case missingReadingOrder(elementId: UInt32) // Element has no reading order

    public var id: String {
        switch self {
        case .tinyBox(let id): return "tiny-\(id)"
        case .outOfBounds(let id): return "oob-\(id)"
        case .lowConfidence(let id): return "conf-\(id)"
        case .extremeAspectRatio(let id): return "aspect-\(id)"
        case .significantOverlap(let e1, let e2): return "overlap-\(e1)-\(e2)"
        case .duplicateReadingOrder(let order, _): return "dup-order-\(order)"
        case .readingOrderGap(let order): return "gap-\(order)"
        case .missingReadingOrder(let id): return "no-order-\(id)"
        }
    }

    var iconName: String {
        switch self {
        case .tinyBox: return "rectangle.dashed"
        case .outOfBounds: return "arrow.up.left.and.arrow.down.right"
        case .lowConfidence: return "exclamationmark.triangle"
        case .extremeAspectRatio: return "aspectratio"
        case .significantOverlap: return "square.on.square"
        case .duplicateReadingOrder: return "list.number"
        case .readingOrderGap: return "list.bullet.indent"
        case .missingReadingOrder: return "questionmark.square"
        }
    }

    var color: Color {
        switch self {
        case .tinyBox: return .orange
        case .outOfBounds: return .red
        case .lowConfidence: return .yellow
        case .extremeAspectRatio: return .purple
        case .significantOverlap: return .blue
        case .duplicateReadingOrder: return .pink
        case .readingOrderGap: return .cyan
        case .missingReadingOrder: return .gray
        }
    }

    var description: String {
        switch self {
        case .tinyBox(let id): return "Element \(id): Very small box"
        case .outOfBounds(let id): return "Element \(id): Out of bounds"
        case .lowConfidence(let id): return "Element \(id): Low confidence"
        case .extremeAspectRatio(let id): return "Element \(id): Extreme aspect ratio"
        case .significantOverlap(let e1, let e2): return "Elements \(e1), \(e2): Significant overlap"
        case .duplicateReadingOrder(let order, let ids):
            return "Order \(order): \(ids.count) duplicates"
        case .readingOrderGap(let order): return "Gap at order \(order)"
        case .missingReadingOrder(let id): return "Element \(id): No reading order"
        }
    }

    var elementIds: [UInt32] {
        switch self {
        case .tinyBox(let id), .outOfBounds(let id),
             .lowConfidence(let id), .extremeAspectRatio(let id),
             .missingReadingOrder(let id):
            return [id]
        case .significantOverlap(let e1, let e2):
            return [e1, e2]
        case .duplicateReadingOrder(_, let ids):
            return ids
        case .readingOrderGap:
            return []  // No specific element for gaps
        }
    }

    /// Human-readable type name for reports
    var typeName: String {
        switch self {
        case .tinyBox: return "Tiny Box"
        case .outOfBounds: return "Out of Bounds"
        case .lowConfidence: return "Low Confidence"
        case .extremeAspectRatio: return "Extreme Aspect Ratio"
        case .significantOverlap: return "Significant Overlap"
        case .duplicateReadingOrder: return "Duplicate Reading Order"
        case .readingOrderGap: return "Reading Order Gap"
        case .missingReadingOrder: return "Missing Reading Order"
        }
    }
}

// MARK: - Validation Summary Label

/// Label for validation section showing issue count
struct ValidationSummaryLabel: View {
    let issueCount: Int

    var body: some View {
        HStack {
            Text("Validation")
            Spacer()
            if issueCount == 0 {
                Image(systemName: "checkmark.circle.fill")
                    .foregroundColor(.green)
                    .font(.caption)
            } else {
                HStack(spacing: 4) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(.orange)
                    Text("\(issueCount)")
                        .foregroundColor(.orange)
                        .monospacedDigit()
                }
                .font(.caption)
            }
        }
    }
}

// MARK: - Validation View

/// View showing annotation validation issues
struct ValidationView: View {
    let elements: [Element]
    let pageSize: (width: Float, height: Float)
    let onSelectElement: (UInt32) -> Void

    /// Static helper to compute issue count without full view
    static func computeIssueCount(elements: [Element], pageSize: (width: Float, height: Float)) -> Int {
        var count = 0

        for element in elements {
            // Tiny boxes
            if element.bbox.width < 10 || element.bbox.height < 10 { count += 1 }
            // Out of bounds
            if element.bbox.x < 0 || element.bbox.y < 0 ||
               element.bbox.x + element.bbox.width > pageSize.width ||
               element.bbox.y + element.bbox.height > pageSize.height { count += 1 }
            // Low confidence
            if element.confidence < 0.5 { count += 1 }
            // Extreme aspect ratio
            let aspectRatio = element.bbox.width / max(element.bbox.height, 0.001)
            if aspectRatio < 0.05 || aspectRatio > 20 { count += 1 }
        }

        // Overlap check (limited to first 100)
        let checkElements = Array(elements.prefix(100))
        for i in 0..<checkElements.count {
            for j in (i+1)..<checkElements.count {
                let b1 = checkElements[i].bbox
                let b2 = checkElements[j].bbox
                let x1 = max(b1.x, b2.x)
                let y1 = max(b1.y, b2.y)
                let x2 = min(b1.x + b1.width, b2.x + b2.width)
                let y2 = min(b1.y + b1.height, b2.y + b2.height)
                if x1 < x2 && y1 < y2 {
                    let overlapArea = (x2 - x1) * (y2 - y1)
                    let area1 = b1.width * b1.height
                    let area2 = b2.width * b2.height
                    if area1 > 0 && area2 > 0 {
                        let ratio = max(overlapArea / area1, overlapArea / area2)
                        if ratio > 0.5 { count += 1 }
                    }
                }
            }
        }

        // Reading order validation
        let withOrder = elements.filter { $0.hasReadingOrder }
        let withoutOrder = elements.filter { !$0.hasReadingOrder }

        // Missing reading order (only if some have it)
        if !withOrder.isEmpty && !withoutOrder.isEmpty {
            count += withoutOrder.count
        }

        // Duplicates
        var orderCounts: [Int32: Int] = [:]
        for element in withOrder {
            orderCounts[element.readingOrder, default: 0] += 1
        }
        for (_, c) in orderCounts where c > 1 {
            count += 1  // One issue per duplicate order number
        }

        // Gaps
        if !withOrder.isEmpty {
            let orders = Set(withOrder.map { $0.readingOrder })
            if let minOrder = orders.min(), let maxOrder = orders.max() {
                for expected in minOrder...maxOrder {
                    if !orders.contains(expected) { count += 1 }
                }
            }
        }

        return count
    }

    /// All detected validation issues
    var issues: [ValidationIssue] {
        var result: [ValidationIssue] = []

        for element in elements {
            // Check for tiny boxes (< 10 pt in either dimension)
            if element.bbox.width < 10 || element.bbox.height < 10 {
                result.append(.tinyBox(elementId: element.id))
            }

            // Check for out of bounds
            if element.bbox.x < 0 || element.bbox.y < 0 ||
               element.bbox.x + element.bbox.width > pageSize.width ||
               element.bbox.y + element.bbox.height > pageSize.height {
                result.append(.outOfBounds(elementId: element.id))
            }

            // Check for low confidence
            if element.confidence < 0.5 {
                result.append(.lowConfidence(elementId: element.id))
            }

            // Check for extreme aspect ratios (< 0.05 or > 20)
            let aspectRatio = element.bbox.width / max(element.bbox.height, 0.001)
            if aspectRatio < 0.05 || aspectRatio > 20 {
                result.append(.extremeAspectRatio(elementId: element.id))
            }
        }

        // Check for significant overlaps (> 50%)
        // Only check first 100 elements to avoid O(n²) slowdown
        let checkElements = Array(elements.prefix(100))
        for i in 0..<checkElements.count {
            for j in (i+1)..<checkElements.count {
                let e1 = checkElements[i]
                let e2 = checkElements[j]

                if let overlapRatio = computeOverlapRatio(e1.bbox, e2.bbox),
                   overlapRatio > 0.5 {
                    result.append(.significantOverlap(element1: e1.id, element2: e2.id))
                }
            }
        }

        // Reading order validation
        let readingOrderIssues = validateReadingOrder(elements: elements)
        result.append(contentsOf: readingOrderIssues)

        return result
    }

    /// Validate reading order for issues
    private func validateReadingOrder(elements: [Element]) -> [ValidationIssue] {
        var issues: [ValidationIssue] = []

        // Get elements with and without reading order
        let withOrder = elements.filter { $0.hasReadingOrder }
        let withoutOrder = elements.filter { !$0.hasReadingOrder }

        // If some elements have reading order but others don't, flag missing ones
        if !withOrder.isEmpty && !withoutOrder.isEmpty {
            for element in withoutOrder {
                issues.append(.missingReadingOrder(elementId: element.id))
            }
        }

        // Check for duplicates
        var orderMap: [Int32: [UInt32]] = [:]
        for element in withOrder {
            orderMap[element.readingOrder, default: []].append(element.id)
        }
        for (order, elementIds) in orderMap where elementIds.count > 1 {
            issues.append(.duplicateReadingOrder(order: order, elements: elementIds))
        }

        // Check for gaps in sequence
        if !withOrder.isEmpty {
            let orders = Set(withOrder.map { $0.readingOrder })
            if let minOrder = orders.min(), let maxOrder = orders.max() {
                for expected in minOrder...maxOrder {
                    if !orders.contains(expected) {
                        issues.append(.readingOrderGap(expectedOrder: expected))
                    }
                }
            }
        }

        return issues
    }

    /// Issues grouped by type for summary
    var issuesByType: [(type: String, count: Int, color: Color)] {
        var tiny = 0, oob = 0, lowConf = 0, aspect = 0, overlap = 0
        var dupOrder = 0, orderGap = 0, noOrder = 0

        for issue in issues {
            switch issue {
            case .tinyBox: tiny += 1
            case .outOfBounds: oob += 1
            case .lowConfidence: lowConf += 1
            case .extremeAspectRatio: aspect += 1
            case .significantOverlap: overlap += 1
            case .duplicateReadingOrder: dupOrder += 1
            case .readingOrderGap: orderGap += 1
            case .missingReadingOrder: noOrder += 1
            }
        }

        var result: [(type: String, count: Int, color: Color)] = []
        if tiny > 0 { result.append(("Tiny boxes", tiny, .orange)) }
        if oob > 0 { result.append(("Out of bounds", oob, .red)) }
        if lowConf > 0 { result.append(("Low confidence", lowConf, .yellow)) }
        if aspect > 0 { result.append(("Extreme aspect", aspect, .purple)) }
        if overlap > 0 { result.append(("Overlaps", overlap, .blue)) }
        if dupOrder > 0 { result.append(("Duplicate order", dupOrder, .pink)) }
        if orderGap > 0 { result.append(("Order gaps", orderGap, .cyan)) }
        if noOrder > 0 { result.append(("Missing order", noOrder, .gray)) }
        return result
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if issues.isEmpty {
                // No issues found - show success
                HStack {
                    Image(systemName: "checkmark.circle.fill")
                        .foregroundColor(.green)
                    Text("No issues found")
                        .font(.caption)
                        .foregroundColor(.green)
                }
            } else {
                // Summary by type
                HStack {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(.orange)
                    Text("\(issues.count) issue\(issues.count == 1 ? "" : "s") found")
                        .font(.caption.bold())
                        .foregroundColor(.orange)
                }

                // Issue type breakdown
                ForEach(issuesByType, id: \.type) { item in
                    HStack(spacing: 6) {
                        Circle()
                            .fill(item.color)
                            .frame(width: 6, height: 6)
                        Text(item.type)
                            .font(.caption)
                        Spacer()
                        Text("\(item.count)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .monospacedDigit()
                    }
                }

                Divider()

                // Scrollable list of issues (max 10)
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(Array(issues.prefix(10))) { issue in
                        Button(action: {
                            // Select first element of this issue
                            if let firstId = issue.elementIds.first {
                                onSelectElement(firstId)
                            }
                        }) {
                            HStack(spacing: 6) {
                                Image(systemName: issue.iconName)
                                    .foregroundColor(issue.color)
                                    .frame(width: 14)
                                Text(issue.description)
                                    .font(.caption2)
                                    .foregroundColor(.primary)
                                    .lineLimit(1)
                                Spacer()
                            }
                        }
                        .buttonStyle(.plain)
                    }

                    if issues.count > 10 {
                        Text("... and \(issues.count - 10) more")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
    }

    /// Compute overlap ratio between two bounding boxes
    /// Returns nil if no overlap, otherwise returns max(overlap/area1, overlap/area2)
    private func computeOverlapRatio(_ b1: BoundingBox, _ b2: BoundingBox) -> Float? {
        let x1 = max(b1.x, b2.x)
        let y1 = max(b1.y, b2.y)
        let x2 = min(b1.x + b1.width, b2.x + b2.width)
        let y2 = min(b1.y + b1.height, b2.y + b2.height)

        // No overlap
        if x1 >= x2 || y1 >= y2 { return nil }

        let overlapArea = (x2 - x1) * (y2 - y1)
        let area1 = b1.width * b1.height
        let area2 = b2.width * b2.height

        // Avoid division by zero
        guard area1 > 0 && area2 > 0 else { return nil }

        return max(overlapArea / area1, overlapArea / area2)
    }
}
