import Foundation

/// Equal collation values share a rank so a later key can break the tie.
/// The bridge returns ranks only; document rows never cross the native boundary.
enum FolioTextSort {
  static func ranks(_ values: [[String]], locale: String, caseSensitive: Bool) -> [[Int]] {
    let language = Locale(identifier: locale)
    let options: String.CompareOptions = caseSensitive ? [] : [.caseInsensitive]
    return values.map { column in
      func compare(_ a: Int, _ b: Int) -> ComparisonResult {
        column[a].compare(column[b], options: options, range: nil, locale: language)
      }
      let order = column.indices.sorted {
        let result = compare($0, $1)
        return result == .orderedSame ? $0 < $1 : result == .orderedAscending
      }
      var ranks = Array(repeating: 0, count: column.count)
      var rank = 0
      for i in order.indices {
        if i > 0 && compare(order[i - 1], order[i]) != .orderedSame { rank += 1 }
        ranks[order[i]] = rank
      }
      return ranks
    }
  }
}
