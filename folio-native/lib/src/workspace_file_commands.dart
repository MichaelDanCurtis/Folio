import 'package:flutter/widgets.dart';
import 'package:flutter/services.dart';

enum WorkspaceFileCommand {
  newWorkspace,
  open,
  recent,
  fromTemplate,
  save,
  saveAs,
  saveCopy,
  saveTemplate,
  export,
  exportPdf,
  compatibility,
  newTab,
  openInNewTab,
  closeTab,
}

class WorkspaceFileIntent extends Intent {
  const WorkspaceFileIntent(this.command);
  final WorkspaceFileCommand command;
}

Map<ShortcutActivator, Intent> workspaceFileShortcuts({
  bool reserveControlShiftE = false,
}) => {
  for (final meta in [true, false]) ...{
    SingleActivator(LogicalKeyboardKey.keyN, meta: meta, control: !meta):
        const WorkspaceFileIntent(WorkspaceFileCommand.newWorkspace),
    SingleActivator(LogicalKeyboardKey.keyO, meta: meta, control: !meta):
        const WorkspaceFileIntent(WorkspaceFileCommand.open),
    SingleActivator(LogicalKeyboardKey.keyT, meta: meta, control: !meta):
        const WorkspaceFileIntent(WorkspaceFileCommand.newTab),
    SingleActivator(
      LogicalKeyboardKey.keyO,
      meta: meta,
      control: !meta,
      alt: true,
    ): const WorkspaceFileIntent(
      WorkspaceFileCommand.openInNewTab,
    ),
    SingleActivator(LogicalKeyboardKey.keyS, meta: meta, control: !meta):
        const WorkspaceFileIntent(WorkspaceFileCommand.save),
    SingleActivator(
      LogicalKeyboardKey.keyS,
      meta: meta,
      control: !meta,
      shift: true,
    ): const WorkspaceFileIntent(
      WorkspaceFileCommand.saveAs,
    ),
    if (meta || !reserveControlShiftE)
      SingleActivator(
        LogicalKeyboardKey.keyE,
        meta: meta,
        control: !meta,
        shift: true,
      ): const WorkspaceFileIntent(
        WorkspaceFileCommand.export,
      ),
  },
};

const fileMenuChannel = MethodChannel('folio/file-menu');
