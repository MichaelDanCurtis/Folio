#!/usr/bin/env python3
"""Exercise production history-menu objects without posting desktop events."""
from pathlib import Path
import subprocess,tempfile
root=Path(__file__).resolve().parents[1]
s=(root/'macos/Runner/MainFlutterWindow.swift').read_text()
s=s[s.index('final class FolioHistoryMenu:'):s.index('class FolioFlutterViewController:')]
checks=r'''
final class TestWindow: NSWindow {
 var simulatedVisible=true
 var simulatedSheet: NSWindow?
 override var isVisible: Bool { simulatedVisible }
 override var attachedSheet: NSWindow? { simulatedSheet }
}
_ = NSApplication.shared
NSApp.mainMenu=NSMenu(title:"Main")
let top=NSMenuItem(title:"Edit",action:nil,keyEquivalent:"")
let menu=NSMenu(title:"Edit");top.submenu=menu;NSApp.mainMenu!.addItem(top)
for (title,action,key,flags) in [("Undo","undo:","z",NSEvent.ModifierFlags.command),("Redo","redo:","z",[.command,.shift]),("Copy","copy:","c",.command)] {
 let item=NSMenuItem(title:title,action:NSSelectorFromString(action),keyEquivalent:key)
 item.keyEquivalentModifierMask=flags;menu.addItem(item)
}
let undo=menu.items[0],redo=menu.items[1],copy=menu.items[2]
let window=TestWindow(contentRect:.zero,styleMask:[],backing:.buffered,defer:true)
let history=FolioHistoryMenu(window:window)
var calls:[String]=[]
history.onCommand={command,generation in calls.append("\(command):\(generation)")}
history.update(available:true,canUndo:true,canRedo:false,generation:7)
precondition(undo.target === history && redo.target === history)
precondition(copy.target == nil && copy.action == NSSelectorFromString("copy:"))
precondition(history.validateMenuItem(undo) && !history.validateMenuItem(redo))
menu.performActionForItem(at:0);menu.performActionForItem(at:1)
precondition(calls == ["undo:7"])
func event(_ flags:NSEvent.ModifierFlags,_ key:String="z") -> NSEvent {
 NSEvent.keyEvent(with:.keyDown,location:.zero,modifierFlags:flags,timestamp:0,windowNumber:window.windowNumber,context:nil,characters:key,charactersIgnoringModifiers:key,isARepeat:false,keyCode:6)!
}
precondition(history.performKeyEquivalent(with:event(.command)))
precondition(!history.performKeyEquivalent(with:event([.command,.shift])))
precondition(!history.performKeyEquivalent(with:event([.command,.option])))
precondition(!history.performKeyEquivalent(with:event(.command,"s")))
history.update(available:true,canUndo:false,canRedo:true,generation:8)
precondition(history.performKeyEquivalent(with:event([.command,.shift])))
precondition(calls == ["undo:7","undo:7","redo:8"])
// Native panels must see the original responder-chain actions, not Dart history.
window.simulatedSheet=NSWindow(contentRect:.zero,styleMask:[],backing:.buffered,defer:true)
history.refresh()
precondition(undo.target == nil && undo.action == NSSelectorFromString("undo:"))
precondition(redo.target == nil && redo.action == NSSelectorFromString("redo:"))
precondition(!history.performKeyEquivalent(with:event([.command,.shift])))
window.simulatedSheet=nil;history.refresh()
precondition(redo.target === history && history.validateMenuItem(redo))
window.simulatedVisible=false;precondition(!history.validateMenuItem(redo))
window.simulatedVisible=true
history.update(available:false,canUndo:true,canRedo:true,generation:9)
precondition(!history.validateMenuItem(undo) && !history.validateMenuItem(redo))
precondition(menu.items.map(\.title)==["Undo","Redo","Copy"])
precondition(undo.keyEquivalent=="z" && undo.keyEquivalentModifierMask == .command)
precondition(redo.keyEquivalentModifierMask == [.command,.shift])
print("Native history menu: availability, routing, modifiers, generations and native-panel ownership passed; no desktop events posted.")
'''
with tempfile.TemporaryDirectory(prefix='folio-history-menu-') as folder:
 p=Path(folder)/'main.swift';p.write_text('import Cocoa\n'+s+checks)
 subprocess.run(['xcrun','--sdk','macosx','swift',str(p)],check=True)
