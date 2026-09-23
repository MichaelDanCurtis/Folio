import 'dart:io';
import 'package:flutter/services.dart';
import 'recent_files.dart';
import 'working_files.dart';

class OfficeTemplate {
  const OfficeTemplate(
    this.id,
    this.title,
    this.description,
    this.kind, {
    this.asset,
    this.file,
  });
  final String id, title, description;
  final WorkingFileKind kind;
  final String? asset;
  final RecentFile? file;
}

const starterTemplates = [
  OfficeTemplate(
    'letter',
    'Business letter',
    'A letter with sender, recipient and closing placeholders.',
    WorkingFileKind.docx,
    asset: 'letter.dotx',
  ),
  OfficeTemplate(
    'report',
    'Project report',
    'Summary, progress, results and next steps.',
    WorkingFileKind.docx,
    asset: 'report.dotx',
  ),
  OfficeTemplate(
    'minutes',
    'Meeting notes',
    'Participants, decisions and an action-item table.',
    WorkingFileKind.docx,
    asset: 'minutes.dotx',
  ),
  OfficeTemplate(
    'budget',
    'Budget',
    'Planned and actual amounts, remaining funds and totals.',
    WorkingFileKind.xlsx,
    asset: 'budget.xltx',
  ),
  OfficeTemplate(
    'invoice',
    'Invoice',
    'Line items, quantities, prices, tax and totals.',
    WorkingFileKind.xlsx,
    asset: 'invoice.xltx',
  ),
  OfficeTemplate(
    'tracker',
    'Project tracker',
    'Tasks, owners, due dates and a status dropdown.',
    WorkingFileKind.xlsx,
    asset: 'tracker.xltx',
  ),
];

class TemplateLibrary {
  TemplateLibrary(this.directory, this.access)
    : files = RecentFiles('$directory/library.json', unpinnedLimit: null);
  final String directory;
  final WorkingFileAccess access;
  final RecentFiles files;
  List<OfficeTemplate> get entries => [
    ...starterTemplates,
    for (final file in files.entries)
      OfficeTemplate(
        file.key,
        file.name.replaceFirst(
          RegExp(r'\.(dotx|xltx)$', caseSensitive: false),
          '',
        ),
        file.path,
        file.kind,
        file: file,
      ),
  ];
  Future<void> load() => files.load();
  Future<FileAccessLease> acquire(OfficeTemplate template) async {
    if (template.file != null) {
      return access.acquireLocation(
        template.file!.path,
        template.file!.bookmark,
      );
    }
    final bytes = await rootBundle.load('assets/templates/${template.asset}');
    final root = Directory('$directory/Starters');
    await root.create(recursive: true);
    final temporary = await root.createTemp('preview-');
    final file = File('${temporary.path}/${template.asset}');
    try {
      await file.writeAsBytes(
        bytes.buffer.asUint8List(bytes.offsetInBytes, bytes.lengthInBytes),
        flush: true,
      );
    } catch (_) {
      await temporary.delete(recursive: true);
      rethrow;
    }
    return FileAccessLease(file.path, null, () async {
      await temporary.delete(recursive: true);
    });
  }

  Future<void> remember(String path, {String? previousPath}) async {
    final extension = path.split('.').last.toLowerCase();
    if (!['dotx', 'xltx'].contains(extension)) {
      throw ArgumentError('Choose a DOTX or XLTX template.');
    }
    await files.remember(
      workingFileKindForPath(path),
      path,
      await access.capture(path),
      previousPath: previousPath,
    );
  }
}

class TemplateSelection {
  const TemplateSelection(this.template, this.fingerprint, {this.edit = false});
  final OfficeTemplate template;
  final String fingerprint;
  final bool edit;
}
