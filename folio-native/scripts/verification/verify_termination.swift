import Cocoa

@main
struct VerifyTermination {
  static func main() {
    let coordinator = FolioTermination()
    var finish: ((Bool) -> Void)?
    var saves = 0, exits = 0
    func request() -> NSApplication.TerminateReply {
      coordinator.request(prepare: { completion in
        saves += 1
        finish = completion
      }, terminate: {
        exits += 1
        precondition(request() == .terminateNow)
      })
    }
    func drain() { RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.02)) }
    precondition(request() == .terminateCancel)
    precondition(request() == .terminateCancel)
    precondition(saves == 1 && exits == 0)
    finish!(false)
    drain()
    precondition(exits == 0)
    precondition(request() == .terminateCancel)
    precondition(saves == 2)
    finish!(true)
    precondition(exits == 0)
    precondition(request() == .terminateCancel)
    drain()
    precondition(exits == 1 && saves == 2)

    let synchronous = FolioTermination()
    var returned = false
    precondition(synchronous.request(prepare: { $0(true) }, terminate: {
      precondition(returned)
      exits += 1
    }) == .terminateCancel)
    returned = true
    drain()
    precondition(exits == 2)
    print("Quit waits for saving, coalesces repeats, retries failure and re-enters only after approval.")
  }
}
