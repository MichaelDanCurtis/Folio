import Foundation

@main
struct VerifyTextSort {
  static func main() {
    let canonical = FolioTextSort.ranks([["é", "e\u{301}", "E", "e"]], locale: "fr_FR", caseSensitive: false)[0]
    precondition(canonical[0] == canonical[1] && canonical[2] == canonical[3])
    precondition(canonical[2] < canonical[0])
    let swedish = FolioTextSort.ranks([["ö", "z", "ä", "å"]], locale: "sv_SE", caseSensitive: false)[0]
    precondition(swedish == [3, 0, 2, 1])
    let german = FolioTextSort.ranks([["ä", "z"]], locale: "de_DE", caseSensitive: false)[0]
    precondition(german == [0, 1])
    let turkish = FolioTextSort.ranks([["I", "ı", "İ", "i"]], locale: "tr_TR", caseSensitive: false)[0]
    precondition(turkish == [0, 0, 1, 1])
    let sensitive = FolioTextSort.ranks([["a", "A"], ["", "", "same"]], locale: "en_US", caseSensitive: true)
    precondition(sensitive[0][0] != sensitive[0][1])
    precondition(sensitive[1] == [0, 0, 1])
    precondition(FolioTextSort.ranks([[]], locale: "en_US", caseSensitive: false) == [[]])
    print("Native Foundation collation: canonical equivalence, case, Swedish, German, Turkish, empty and multiple columns pass.")
  }
}
