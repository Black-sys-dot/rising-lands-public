import Foundation

// Standalone headless executable. Compile ONLY with NativeBenchmarkArguments.swift.
@main
struct BenchmarkArgumentsTests {
    static func main() {
        var checks = 0
        func expect(_ condition: Bool, _ label: String) {
            checks += 1
            if !condition { FileHandle.standardError.write(Data("BenchmarkArgumentsTests FAILED: \(label)\n".utf8)); exit(1) }
        }
        typealias A = NativeBenchmarkArguments
        func parse(_ args: [String], _ mode: String = "play") -> A? { try? A.parse(args, mode: mode) }
        expect(parse([]) == A(mission: "MONDE01.ETA", difficulty: 1), "default MONDE01 Normal")
        expect(parse(["--benchmark-mission", "monde14.eta"]) == A(mission: "MONDE14.ETA", difficulty: 1), "stage name normalized")
        expect(parse(["--benchmark-mission", "MONDE17A.ETA", "--benchmark-difficulty", "2"]) == A(mission: "MONDE17A.ETA", difficulty: 2), "variant and difficulty")
        for bad in ["MONDE26.ETA", "MONDE00.ETA", "MONDE1.ETA", "MONDE02E.ETA", "../MONDE02.ETA", ""] {
            expect(parse(["--benchmark-mission", bad]) == nil, "rejects \(bad)")
        }
        expect(parse(["--benchmark-mission"]) == nil, "missing value")
        expect(parse(["--benchmark-difficulty", "3"]) == nil && parse(["--benchmark-difficulty", "x"]) == nil, "difficulty range")
        expect(parse(["--benchmark-mission", "MONDE02.ETA"], "test") == nil && parse([], "test") != nil, "test mode stays MONDE01 Normal")
        expect(parse(["--benchmark-orders", "--benchmark-mission", "MONDE06.ETA"]) == nil, "orders only for MONDE01")
        expect(parse(["--benchmark-orders", "--benchmark-orders-selected"]) != nil, "MONDE01 orders accepted")
        print("BenchmarkArgumentsTests: \(checks) checks passed; stage/difficulty selection, test and order restrictions")
    }
}
