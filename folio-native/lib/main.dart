import 'src/document_page_preview.dart';
import 'src/formula_inspector.dart';
import 'src/goal_seek_dialog.dart';
import 'src/scenario_manager.dart';
import 'src/watch_window.dart';
import 'src/document_columns.dart';
import 'src/working_files.dart';
import 'src/recent_files.dart';
import 'src/file_browser.dart';
import 'src/templates.dart';
import 'src/template_gallery.dart';
import 'src/workspace_file_commands.dart';
import 'src/document_text_style.dart';
import 'src/document_paragraph_format.dart';
import 'src/document_page_setup.dart';
import 'src/document_sections.dart';
import 'src/document_headers.dart';
import 'src/draft_text_field.dart';
import 'src/history_commands.dart';
import 'dart:async';
import 'src/document_tabs.dart';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' show AppExitResponse;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_quill/flutter_quill.dart' as q;
import 'package:file_selector/file_selector.dart';
import 'package:path_provider/path_provider.dart';
import 'src/calendar_refresh.dart';
import 'src/core.dart';
import 'src/agent.dart';
import 'src/document_codec.dart';
import 'src/document_bookmarks.dart';
import 'src/document_comments.dart';
import 'src/document_revisions.dart';
import 'src/document_review_view.dart';
import 'src/document_tracking.dart';
import 'src/document_bookmarks_dialog.dart';
import 'src/document_edit_tracking.dart';
import 'src/document_selection.dart';
import 'src/document_history_input.dart';
import 'src/document_history_change.dart';
import 'package:dart_quill_delta/dart_quill_delta.dart';
import 'src/document_view.dart';
import 'src/sheet_view.dart';
import 'src/sheet_status.dart';
import 'src/table_controls.dart' show VisibleRows, VisibleColumns;
import 'src/sheet_reviews.dart';
import 'src/document_anchor.dart';
import 'src/assistant_ui.dart';
import 'src/workbook_block.dart';
import 'src/proofreading.dart';
import 'src/writing_panel.dart';
import 'src/search.dart';
import 'src/search_panel.dart';

const ink = Color(0xff242424),
    muted = Color(0xff616161),
    line = Color(0xffdfe3e8),
    paper = Color(0xffffffff),
    accent = Color(0xff185abd);
const lifecycleChannel = MethodChannel('folio/lifecycle');
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  try {
    await CoreBridge.initialize();
    final dir =
        Platform.environment['FOLIO_TEST_DATA'] ??
        '${(await getApplicationSupportDirectory()).path}/FolioNative';
    await Directory(dir).create(recursive: true);
    final model = WorkspaceModel(CoreBridge(), '$dir/workspace.folio');
    await model.initialize();
    runApp(FolioApp(model: model, agentDirectory: '$dir/agent'));
  } catch (e) {
    runApp(
      MaterialApp(
        home: Scaffold(
          body: Center(
            child: Padding(
              padding: const EdgeInsets.all(40),
              child: SelectableText(
                'Folio could not open its native engine.\n\n$e',
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class FolioApp extends StatelessWidget {
  final WorkspaceModel model;
  final String agentDirectory;
  const FolioApp({
    super.key,
    required this.model,
    required this.agentDirectory,
  });
  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'Folio Native',
    builder: (context, child) => AnimatedBuilder(
      animation: model,
      builder: (context, _) => DocumentTabSettings(
        intervalTwips: documentTabInterval(model.workspace['document']),
        child: q.QuillParagraphLayout(
          builder: layoutDocumentTabs,
          paddingBuilder: paragraphPadding,
          styleBuilder: paragraphTextStyle,
          inlineStyleBuilder: paragraphInlineStyle,
          alignmentBuilder: paragraphAlignment,
          strutBuilder: paragraphStrut,
          blockSpacingBuilder: paragraphBlockSpacing,
          child: child!,
        ),
      ),
    ),
    debugShowCheckedModeBanner: false,
    localizationsDelegates: const [
      GlobalMaterialLocalizations.delegate,
      GlobalCupertinoLocalizations.delegate,
      GlobalWidgetsLocalizations.delegate,
      q.FlutterQuillLocalizations.delegate,
    ],
    theme: ThemeData(
      useMaterial3: true,
      brightness: Brightness.light,
      scaffoldBackgroundColor: const Color(0xfff8f7f3),
      colorScheme: ColorScheme.fromSeed(seedColor: accent, surface: paper),
      fontFamily: '.AppleSystemUIFont',
      dividerColor: line,
      textTheme: const TextTheme(
        bodyMedium: TextStyle(color: ink, fontSize: 13),
        bodySmall: TextStyle(color: muted, fontSize: 11),
      ),
      iconButtonTheme: IconButtonThemeData(
        style: IconButton.styleFrom(
          foregroundColor: const Color(0xff7f7968),
          minimumSize: const Size(30, 30),
          padding: const EdgeInsets.all(6),
        ),
      ),
      tooltipTheme: const TooltipThemeData(
        waitDuration: Duration(milliseconds: 600),
      ),
    ),
    home: WorkspaceScreen(model: model, agentDirectory: agentDirectory),
  );
}

class WorkspaceScreen extends StatefulWidget {
  final WorkspaceModel model;
  final String agentDirectory;
  const WorkspaceScreen({
    super.key,
    required this.model,
    required this.agentDirectory,
  });
  @override
  State<WorkspaceScreen> createState() => WorkspaceScreenState();
}

class WorkspaceScreenState extends State<WorkspaceScreen> {
  WorkspaceModel get model => widget.model;
  late q.QuillController editor;
  late CodexAgent agent;
  late AssistantUi assistantUi;
  late ProofreadingController proofreading;
  WritingReviewSnapshot? writingReview;
  bool writingPanel = false, reviewEntireDocument = false;
  bool searchPanel = false;
  bool watchPanel = false;
  final searchKey = GlobalKey<SearchPanelState>();
  final documentKey = GlobalKey<DocumentViewState>();
  DocumentAnchor? anchor;
  Map<String, dynamic>? attachedRange;
  String ribbon = 'Home';
  Color get officeColor =>
      spreadsheet ? const Color(0xff107c41) : const Color(0xff185abd);
  final prompt = TextEditingController(), chatScroll = ScrollController();
  final _workspaceFocus = FocusScopeNode(debugLabel: 'Folio workspace');
  final sheetKey = GlobalKey<SheetViewState>();
  final proposals = <Map<String, dynamic>>[];
  bool spreadsheet = false, restoring = false;
  bool _fileCommandBusy = false,
      _closePrepared = false,
      _leavingWorkspace = false;
  String? _fileMenuState;
  String? _historyMenuState;
  String? _historyMenuContext;
  bool _historyRouteCurrent = true;
  int _historyMenuGeneration = 0;
  late WorkingFiles workingFiles;
  late RecentFiles recentFiles;
  late RecoveryCatalog recoveryCopies;
  late TemplateLibrary templates;
  String? _templateError;
  String? _recentError;
  late Future<void> filesReady;
  late String sheetId;
  String selectedCell = 'B4', _lastDocument = '';
  Set<SelectionAggregate> _statusAggregates = defaultAggregates;
  int _epoch = 0, _wordCount = 0;
  int _documentSession = 0, _modelRevision = 0;
  Future<void> _documentPending = Future.value();
  Object? _documentError;
  int _documentEditSerial = 0;
  bool _discardingDocumentEdits = false;
  bool _trackingBusy = false;
  bool _reviewingDocument = false;
  bool get recordingNeedsAuthor =>
      model.workspace['officeDocument']?['recordingRequested'] == true &&
      model.workspace['officeDocument']?['tracking'] == null;
  Map? get recording => model.workspace['officeDocument']?['tracking'] as Map?;
  Future<void> _historyPending = Future.value();
  DocumentHistoryInput? _historyInput, _rejectedInput;
  StreamSubscription<q.DocChange>? _inputSubscription;
  Delta? _inputChange;
  late DocumentEditCapture _documentEdits;
  AppLifecycleListener? _lifecycle;
  late CalendarRefresh _calendarRefresh;
  @override
  void initState() {
    super.initState();
    workingFiles = WorkingFiles(
      '${File(model.recoveryPath).parent.path}/working-files.json',
    );
    recentFiles = RecentFiles(
      '${File(model.recoveryPath).parent.path}/recent-files.json',
    );
    recoveryCopies = RecoveryCatalog(
      '${File(model.recoveryPath).parent.path}/Snapshots',
    );
    templates = TemplateLibrary(
      '${File(model.recoveryPath).parent.path}/Templates',
      workingFiles.access,
    );
    // Typing can start while local metadata loads. Preserve the startup
    // baseline now rather than treating those later edits as already saved.
    final initialWorkspace = model.workspace;
    final initialStates = model.snapshot['fileStates'] as Map?;
    final initiallyPristine =
        !model.recovered || model.snapshot['blankWorkspace'] == true;
    filesReady =
        (() async {
              // Migrate locations before blank-workspace initialization can replace
              // the old session's associations after Don't Save and restart.
              try {
                await recentFiles.load(migrateFrom: workingFiles.storePath);
              } catch (e) {
                _recentError = 'Recent files could not be loaded: $e';
              }
              try {
                await templates.load();
              } catch (e) {
                _templateError =
                    'Your template library could not be loaded: $e';
              }
              await workingFiles.load(
                initialWorkspace['id'] as String,
                workspace: initialWorkspace,
                fileStates: initialStates,
                pristine: initiallyPristine,
              );
            })()
            .catchError((Object e) {
              if (mounted) {
                notice(
                  'Saved file locations could not be restored. Use Save As. $e',
                );
              }
            })
            .whenComplete(() {
              if (mounted) setState(() {});
            });
    sheetId = model.sheets.first['id'];
    editor = q.QuillController(
      document: q.Document.fromDelta(
        documentToDelta(model.workspace['document']),
      ),
      selection: const TextSelection.collapsed(offset: 0),
      readOnly: recordingNeedsAuthor,
    );
    _attachDocumentCapture();
    _lastDocument = jsonEncode(
      deltaToDocument(editor.document.toDelta())['content'],
    );
    _wordCount = documentWordCount(model.workspace['document']);
    editor.addListener(_documentChanged);
    proofreading = ProofreadingController(
      editor,
      check: CoreBridge().call,
      settingsPath: '${File(model.recoveryPath).parent.path}/proofreading.json',
    );
    proofreading.addListener(_proofreadingChanged);
    _epoch = model.documentEpoch;
    _documentSession = model.documentSession;
    _modelRevision = model.revision;
    model.addListener(_modelChanged);
    agent = CodexAgent(workingDirectory: widget.agentDirectory, onTool: _tool);
    agent.addListener(_agentChanged);
    assistantUi = AssistantUi(
      model: model,
      onAction: (action) => guard(() => _uiAction(action)),
      onOpen: openSource,
    );
    assistantUi.addListener(_agentChanged);
    fileMenuChannel.setMethodCallHandler((call) async {
      if (call.method != 'command') return null;
      final command = WorkspaceFileCommand.values
          .where((value) => value.name == call.arguments)
          .firstOrNull;
      if (command != null) await guard(() => fileCommand(command));
      return null;
    });
    historyMenuChannel.setMethodCallHandler((call) async {
      if (call.method != 'command' || call.arguments is! Map) return null;
      _syncHistoryMenu();
      final args = call.arguments as Map;
      if (args['generation'] != _historyMenuGeneration ||
          !['undo', 'redo'].contains(args['command'])) {
        return null;
      }
      final state = _historyState();
      final redo = args['command'] == 'redo';
      if (state['available'] != true ||
          state[redo ? 'canRedo' : 'canUndo'] != true) {
        return null;
      }
      await guard(() => _invokeHistory(redo));
      _syncHistoryMenu();
      return null;
    });
    FocusManager.instance.addListener(_syncHistoryMenu);
    focusedDraftHistory.addListener(_syncHistoryMenu);
    lifecycleChannel.setMethodCallHandler((call) async {
      if (call.method == 'prepareClose') return prepareClose();
      return null;
    });
    _calendarRefresh = CalendarRefresh(
      onChange: () => model.refreshCalculations(),
      onError: (e) {
        if (mounted) notice('Date-based calculations could not refresh: $e');
      },
    )..start();
    _lifecycle = AppLifecycleListener(
      onResume: () => unawaited(
        _calendarRefresh.check().catchError((Object e) {
          if (mounted) notice(e.toString());
        }),
      ),
      onExitRequested: () async {
        try {
          return await prepareClose()
              ? AppExitResponse.exit
              : AppExitResponse.cancel;
        } catch (error) {
          notice('Folio could not finish saving. $error');
          return AppExitResponse.cancel;
        }
      },
    );
  }

  Future<void> recalculate() async {
    await flush();
    await model.refreshCalculations(force: true);
  }

  void _attachDocumentCapture() {
    _documentEdits = DocumentEditCapture.attach(editor.document);
    _inputSubscription?.cancel();
    _inputChange = null;
    _inputSubscription = editor.document.synchronousChanges.listen((event) {
      if ((_historyInput != null || _rejectedInput != null) && !restoring) {
        _inputChange = (_inputChange ?? Delta()).compose(event.change);
      }
    });
  }

  void _documentChanged() {
    if (restoring) return;
    final doc = deltaToDocument(
      editor.document.toDelta(),
      attributes: model.workspace['document']['attrs'] as Map?,
    );
    final encoded = jsonEncode(doc['content']);
    final edits = _documentEdits.hasChanges
        ? _documentEdits.take({
            'type': 'doc',
            'content': jsonDecode(_lastDocument),
          }, doc)
        : null;
    final inputChange = _inputChange;
    _inputChange = null;
    if (encoded == _lastDocument) return;
    _lastDocument = encoded;
    _wordCount = documentWordCount(doc);
    final historyInput = _historyInput ?? _rejectedInput;
    if (historyInput != null) {
      historyInput.commands.add(
        QueuedDocumentEdit(
          inputChange ?? historyInput.visible.diff(editor.document.toDelta()),
          edits,
        ),
      );
      if (_historyInput == null) {
        // Retry the exact rejected operations before submitting the next edit.
        // A snapshot-only retry would lose their bookmark journals.
        _rejectedInput = null;
        _documentPending = Future.value();
        _historyInput = historyInput;
        _historyPending = _drainHistoryInput(historyInput);
      }
      return;
    }
    final serial = ++_documentEditSerial;
    _documentPending = model
        .mutate({
          'op': 'document',
          'inheritStyleCatalog': true,
          'document': doc,
          'textEdits': ?edits,
        })
        .then((_) {
          if (mounted &&
              serial == _documentEditSerial &&
              _documentError != null) {
            setState(() => _documentError = null);
          }
        });
    // Surface the error, while retaining it so saving/closing cannot silently
    // succeed with an older document if the native engine rejects an edit.
    unawaited(
      _documentPending.catchError((Object e) {
        if (!mounted || serial != _documentEditSerial) return;
        setState(() => _documentError = e);
        model.recordDocumentSaveFailure(e);
      }),
    );
  }

  void _modelChanged() {
    if (!mounted) return;
    if (_documentSession != model.documentSession) _reviewingDocument = false;
    editor.readOnly =
        recordingNeedsAuthor || _discardingDocumentEdits || _reviewingDocument;
    final previousRevision = _modelRevision;
    _modelRevision = model.revision;
    if (_epoch != model.documentEpoch) {
      restoring = true;
      final committed = documentToDelta(model.workspace['document']);
      final hint = model.snapshot['documentHistoryChange'];
      final exactChange =
          hint is Map &&
              hint['workspaceId'] == model.workspace['id'] &&
              hint['beforeRevision'] == previousRevision &&
              hint['afterRevision'] == model.revision
          ? exactDocumentHistoryChange(
              _historyInput?.base ?? editor.document.toDelta(),
              committed,
              hint['textEdits'],
            )
          : null;
      final inputChange =
          _historyInput?.rebase(committed, change: exactChange) ?? exactChange;
      final nextDocument = q.Document.fromDelta(
        _historyInput?.visible ?? committed,
      );
      final selection = _documentSession == model.documentSession
          ? selectionAfterDocumentRefresh(
              editor.document,
              nextDocument,
              editor.selection,
              change: inputChange,
            )
          : const TextSelection.collapsed(offset: 0);
      editor.document = nextDocument;
      _attachDocumentCapture();
      editor.updateSelection(selection, q.ChangeSource.local);
      _lastDocument = jsonEncode(
        deltaToDocument(editor.document.toDelta())['content'],
      );
      _wordCount = documentWordCount(deltaToDocument(nextDocument.toDelta()));
      _epoch = model.documentEpoch;
      _documentSession = model.documentSession;
      restoring = false;
    }
    if (!model.sheets.any((s) => s['id'] == sheetId)) {
      sheetId = model.sheets.first['id'];
    }
    setState(() {});
  }

  void _agentChanged() {
    if (!mounted) return;
    setState(() {});
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (chatScroll.hasClients) {
        chatScroll.jumpTo(chatScroll.position.maxScrollExtent);
      }
    });
  }

  void _proofreadingChanged() {
    if (mounted) setState(() {});
  }

  void showWritingEditor() {
    setState(() {
      searchPanel = false;
      writingPanel = true;
      spreadsheet = false;
      ribbon = 'Review';
    });
  }

  Future<void> fixWriting(WritingIssue issue, String replacement) async {
    await flush();
    if (!proofreading.isCurrent() ||
        !proofreading.issues.any(
          (current) =>
              current.key == issue.key &&
              current.suggestions.contains(replacement),
        )) {
      throw StateError('The passage changed. Wait for the new writing check.');
    }
    final document = applyWritingEdits(editor, [
      {
        'start': issue.start,
        'end': issue.end,
        'expectedText': issue.text,
        'replacement': replacement,
      },
    ]);
    await model.mutate({
      'op': 'document',
      'document': document,
      'textEdits': _exactDocumentEdits(document, [
        {'start': issue.start, 'end': issue.end, 'text': replacement},
      ]),
    }, refreshDocument: true);
    editor.updateSelection(
      TextSelection.collapsed(offset: issue.start + replacement.length),
      q.ChangeSource.local,
    );
  }

  Future<void> changeSheetGeometry(Map<String, dynamic> action) async {
    await flush();
    await model.mutate({
      'op': 'sheetGeometry',
      'sheetId': sheetId,
      'baseRevision': model.revision,
      ...action,
    }, refreshDocument: true);
  }

  List<Map<String, dynamic>> _exactDocumentEdits(Map after, List<Map> edits) {
    final before = deltaToDocument(editor.document.toDelta());
    final result = checkedTextEdits(
      before,
      after,
      bookmarkEditorEdits(before, edits),
    );
    if (result == null) {
      throw StateError(
        'The document changed. Review the passage and try again.',
      );
    }
    return result;
  }

  Future<void> changeSheetStructure(
    Map<String, dynamic> action, {
    String? target,
  }) async {
    final id = target ?? sheetId;
    await flush();
    await model.mutate({
      'op': 'structure',
      'sheetId': id,
      'baseRevision': model.revision,
      ...action,
    }, refreshDocument: true);
    if (!mounted) return;
    setState(() {
      anchor = null;
      attachedRange = null;
    });
  }

  List<PopupMenuEntry<String>> sheetActions(Map sheet) {
    final index = model.sheets.indexWhere((s) => s['id'] == sheet['id']);
    return [
      const PopupMenuItem(value: 'rename', child: Text('Rename sheet')),
      PopupMenuItem(
        value: 'left',
        enabled: index > 0,
        child: const Text('Move sheet left'),
      ),
      PopupMenuItem(
        value: 'right',
        enabled: index < model.sheets.length - 1,
        child: const Text('Move sheet right'),
      ),
      const PopupMenuDivider(),
      PopupMenuItem(
        value: 'delete',
        enabled: model.sheets.length > 1,
        child: const Text('Delete sheet'),
      ),
    ];
  }

  Future<void> sheetAction(Map sheet, String action) async {
    final id = sheet['id'] as String;
    if (action == 'rename') {
      var draft = sheet['name'] as String;
      final name = await showDialog<String>(
        context: context,
        builder: (dialog) => AlertDialog(
          title: const Text('Rename sheet'),
          content: draftTextField(
            formField: true,
            key: const Key('sheet-name'),
            initialValue: draft,
            autofocus: true,
            onChanged: (value) => draft = value,
            decoration: const InputDecoration(labelText: 'Worksheet name'),
            onFieldSubmitted: (name) => Navigator.pop(dialog, name),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialog),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(dialog, draft),
              child: const Text('Rename'),
            ),
          ],
        ),
      );
      if (name != null && mounted) {
        await changeSheetStructure({
          'action': 'rename',
          'name': name,
        }, target: id);
      }
    } else if (action == 'delete') {
      final remove = await showDialog<bool>(
        context: context,
        builder: (dialog) => AlertDialog(
          title: Text('Delete ${sheet['name']}?'),
          content: const Text(
            'This removes the worksheet and its cells. Formulas and document links to it will show a missing reference. You can restore it with Undo.',
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialog, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(dialog, true),
              child: const Text('Delete'),
            ),
          ],
        ),
      );
      if (remove == true && mounted) {
        await changeSheetStructure({'action': 'delete'}, target: id);
      }
    } else {
      final index = model.sheets.indexWhere((s) => s['id'] == id);
      await changeSheetStructure({
        'action': 'move',
        'index': index + (action == 'left' ? -1 : 1),
      }, target: id);
    }
  }

  Future<void> sheetMenu(Map sheet, Offset position) async {
    final action = await showMenu<String>(
      context: context,
      position: RelativeRect.fromLTRB(
        position.dx,
        position.dy,
        position.dx,
        position.dy,
      ),
      items: sheetActions(sheet),
    );
    if (action != null && mounted) await sheetAction(sheet, action);
  }

  Future<void> checkGrammarWithAi(bool entireDocument) async {
    if (agent.busy) {
      notice('Wait for the assistant to finish, or stop it first.');
      return;
    }
    await flush();
    anchor = DocumentAnchor.capture(
      editor,
      model.revision,
      model.workspace['id'],
    );
    reviewEntireDocument = entireDocument;
    writingReview = null;
    setState(() {
      writingPanel = false;
      searchPanel = false;
      spreadsheet = false;
    });
    prompt.text =
        'Check spelling and grammar in ${entireDocument ? 'the entire document' : 'the selected text or current paragraph'} using ${proofreadingLanguages[proofreading.language]}. '
        'Read read_workspace.proofreading and use proofreadEdits for small, exact corrections. '
        'Keep my meaning, voice, formatting, names, facts and numbers. Do not rewrite for style unless required for grammar. '
        'Explain the corrections briefly and stage one proposal for my review; do not apply it. '
        'If partial is true, say the review covers only the supplied passages. If no corrections are needed, say so.';
    await send(preserveAnchor: true);
  }

  void notice(String message) {
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(message), duration: const Duration(seconds: 5)),
      );
    }
  }

  Future<void> guard(Future<void> Function() work) async {
    try {
      await work();
    } catch (e) {
      notice(e.toString());
    }
  }

  Future<void> flush() async {
    await proofreading.flushSettings();
    await sheetKey.currentState?.flush();
    await _documentPending;
    await _historyPending;
    // The history queue can publish a failed input commit while it drains.
    await _documentPending;
    await model.flush();
  }

  Future<void> history(bool redo) {
    if (_rejectedInput != null) return _documentPending;
    final command = DocumentHistoryCommand(redo);
    final existing = _historyInput;
    if (existing != null) {
      existing.commands.add(command);
    } else {
      // Establish the input boundary synchronously, before saving yields.
      final input = DocumentHistoryInput(editor.document.toDelta());
      input.commands.add(command);
      _historyInput = input;
      _historyPending = _drainHistoryInput(input);
      unawaited(_historyPending.catchError((Object e) => notice(e.toString())));
    }
    return command.done.future;
  }

  Future<void> _drainHistoryInput(DocumentHistoryInput input) async {
    try {
      await _documentPending;
      while (input.commands.isNotEmpty) {
        final command = input.commands.first;
        if (command is DocumentHistoryCommand) {
          try {
            await proofreading.flushSettings();
            await sheetKey.currentState?.flush();
            await model.flush();
            await model.mutate({
              'op': command.redo ? 'redo' : 'undo',
            }, refreshDocument: true);
            command.done.complete();
          } catch (error, stack) {
            // A rejected history request must not discard following input.
            command.done.completeError(error, stack);
          }
        } else if (command is QueuedDocumentEdit) {
          final next = input.base.compose(command.change);
          final document = deltaToDocument(
            next,
            attributes: model.workspace['document']['attrs'] as Map?,
          );
          final edits = checkedTextEdits(
            deltaToDocument(input.base),
            document,
            command.textEdits ?? bookmarkDeltaEdits(input.base, command.change),
          );
          await model.mutate({
            'op': 'document',
            'inheritStyleCatalog': true,
            'document': document,
            'textEdits': ?edits,
          });
          input.base = next;
        }
        input.commands.removeFirst();
      }
      if (mounted && _documentError != null) {
        setState(() => _documentError = null);
      }
    } catch (error, stack) {
      for (final command in input.commands) {
        if (command is DocumentHistoryCommand && !command.done.isCompleted) {
          command.done.completeError(error, stack);
        }
      }
      input.commands.removeWhere(
        (command) => command is DocumentHistoryCommand,
      );
      if (input.commands.isNotEmpty) _rejectedInput = input;
      // Keep the optimistic draft and its exact journals. Save/Close and further
      // history fail until a subsequent document edit successfully retries it.
      model.recordDocumentSaveFailure(error);
      if (mounted) setState(() => _documentError = error);
      _documentPending = Future<void>.error(error, stack);
      unawaited(
        _documentPending.catchError((Object e) => notice(e.toString())),
      );
    } finally {
      _historyInput = null;
      _inputChange = null;
    }
  }

  void historyShortcut(bool redo) {
    guard(() => _invokeHistory(redo));
  }

  Future<void> _invokeHistory(bool redo) async {
    final draft = focusedDraftHistory.value;
    if (draft != null) {
      (redo ? draft.redo : draft.undo)();
      return;
    }
    // Text fields own their drafts, including an empty Undo/Redo stack. Do not
    // flush or undo the document/workbook while one of those fields has focus.
    // Quill uses its own UndoWorkspace action for committed document edits.
    final focused = FocusManager.instance.primaryFocus?.context;
    if (focused != null &&
        focused.findAncestorWidgetOfExactType<q.QuillEditor>() == null) {
      if (redo && Actions.maybeFind<RedoTextIntent>(focused) != null) {
        Actions.maybeInvoke(
          focused,
          const RedoTextIntent(SelectionChangedCause.keyboard),
        );
        return;
      }
      if (!redo && Actions.maybeFind<UndoTextIntent>(focused) != null) {
        Actions.maybeInvoke(
          focused,
          const UndoTextIntent(SelectionChangedCause.keyboard),
        );
        return;
      }
    }
    if (_leavingWorkspace ||
        _fileCommandBusy ||
        !(ModalRoute.of(context)?.isCurrent ?? true)) {
      return;
    }
    await history(redo);
  }

  Map<String, dynamic> _historyState() {
    final draft = focusedDraftHistory.value;
    final focus = FocusManager.instance.primaryFocus;
    final focused = focus?.context;
    final documentFocused =
        focused?.findAncestorWidgetOfExactType<q.QuillEditor>() != null;
    final undoAction = focused == null || documentFocused
        ? null
        : Actions.maybeFind<UndoTextIntent>(focused);
    final redoAction = focused == null || documentFocused
        ? null
        : Actions.maybeFind<RedoTextIntent>(focused);
    final ownsText = draft != null || undoAction != null || redoAction != null;
    return {
      'available':
          !_leavingWorkspace &&
          !_fileCommandBusy &&
          (ownsText || _historyRouteCurrent),
      'canUndo':
          draft?.canUndo ??
          (ownsText
              ? undoAction?.isEnabled(
                      const UndoTextIntent(SelectionChangedCause.keyboard),
                    ) ==
                    true
              : model.canUndo),
      'canRedo':
          draft?.canRedo ??
          (ownsText
              ? redoAction?.isEnabled(
                      const RedoTextIntent(SelectionChangedCause.keyboard),
                    ) ==
                    true
              : model.canRedo),
      'context': ownsText
          ? 'text-${identityHashCode(draft?.owner ?? focus)}'
          : '${model.workspace['id']}:${model.documentSession}:${spreadsheet ? sheetId : 'document'}',
    };
  }

  void _syncHistoryMenu() {
    if (!mounted || !Platform.isMacOS) return;
    final state = _historyState();
    final encoded = jsonEncode(state);
    if (_historyMenuState == encoded) return;
    _historyMenuState = encoded;
    if (_historyMenuContext != state['context']) {
      _historyMenuContext = state['context'] as String;
      _historyMenuGeneration++;
    }
    unawaited(
      historyMenuChannel
          .invokeMethod<void>('state', {
            ...state,
            'generation': _historyMenuGeneration,
          })
          .catchError((Object error) {
            if (error is! MissingPluginException && mounted) {
              notice(error.toString());
            }
          }),
    );
  }

  Future<Map<String, dynamic>> _tool(
    String name,
    Map<String, dynamic> args,
  ) async {
    await flush();
    if (name == 'present_ui') {
      assistantUi.present(args['messages'] as List);
      return {'status': 'shown', 'surfaces': assistantUi.surfaces};
    }
    if (name == 'read_range') {
      return model.core.call({'op': 'readRange', ...args});
    }
    if (name == 'read_workspace') {
      final result = await model.core.call({
        'op': 'context',
        'selection': spreadsheet
            ? {'sheetId': sheetId, 'cell': selectedCell}
            : anchor?.toJson() ??
                  DocumentAnchor.capture(
                    editor,
                    model.revision,
                    model.workspace['id'],
                  ).toJson(),
      });
      result['attachedRange'] = attachedRange;
      writingReview = WritingReviewSnapshot.capture(
        editor,
        model.revision,
        model.workspace['id'],
        start: reviewEntireDocument ? null : anchor?.start,
        end: reviewEntireDocument ? null : anchor?.end,
      );
      result['proofreading'] = writingReview!.toJson();
      return result;
    }
    if (name == 'propose_changes') {
      if (args['proofreadEdits'] is List) {
        if (writingReview == null) {
          throw StateError('Read the workspace before proofreading.');
        }
        if ([
          'replaceSelection',
          'replaceDocument',
          'appendParagraphs',
          'linkedBlock',
          'cells',
          'document',
        ].any(args.containsKey)) {
          throw StateError('Submit proofreading as its own proposal.');
        }
        final edits = writingReview!.resolve(
          editor,
          model.revision,
          model.workspace['id'],
          args['proofreadEdits'],
        );
        args['document'] = applyWritingEdits(editor, edits);
        args['textEdits'] = _exactDocumentEdits(args['document'], [
          for (final e in edits)
            {'start': e['start'], 'end': e['end'], 'text': e['replacement']},
        ]);
      }
      if (args['replaceSelection'] is Map) {
        final selection = anchor;
        if (selection == null) {
          throw StateError('Select a passage before requesting a replacement.');
        }
        args['document'] = selection.replace(
          editor,
          currentRevision: model.revision,
          currentWorkspaceId: model.workspace['id'],
          expectedText: args['replaceSelection']['expectedText'],
          replacement: args['replaceSelection']['text'],
        );
        args['_selectionBefore'] = selection.text;
        args['textEdits'] = _exactDocumentEdits(args['document'], [
          {
            'start': selection.start,
            'end': selection.end,
            'text': args['replaceSelection']['text'],
          },
        ]);
      }
      final p = await model.core.call({'op': 'propose', 'proposal': args});
      p['_before'] = jsonDecode(jsonEncode(model.workspace));
      p['_status'] = 'pending';
      setState(() => proposals.add(p));
      return {
        'status': 'proposed',
        'message':
            'Waiting for the user to review and Apply. No edits have been applied.',
      };
    }
    throw StateError('Unsupported tool: $name');
  }

  Future<void> apply(Map<String, dynamic> p) async {
    await flush();
    await model.mutate({'op': 'apply', 'proposal': p}, refreshDocument: true);
    setState(() => p['_status'] = 'applied');
  }

  bool fileCommandEnabled(WorkspaceFileCommand command) =>
      mounted &&
      !_fileCommandBusy &&
      (ModalRoute.of(context)?.isCurrent ?? true) &&
      (!(command == WorkspaceFileCommand.open ||
              command == WorkspaceFileCommand.fromTemplate ||
              command == WorkspaceFileCommand.recent ||
              command == WorkspaceFileCommand.newWorkspace) ||
          !agent.busy);

  Future<void> fileCommand(WorkspaceFileCommand command) async {
    if (!fileCommandEnabled(command)) return;
    // Claim the operation before the first await: native menus, keys and
    // toolbar clicks cannot enqueue duplicate panels or replace each other.
    final previousFocus = FocusManager.instance.primaryFocus;
    setState(() {
      _fileCommandBusy = true;
      _leavingWorkspace =
          command == WorkspaceFileCommand.newWorkspace ||
          command == WorkspaceFileCommand.fromTemplate ||
          command == WorkspaceFileCommand.recent ||
          command == WorkspaceFileCommand.open;
    });
    try {
      if (_leavingWorkspace) await WidgetsBinding.instance.endOfFrame;
      await filesReady;
      await switch (command) {
        WorkspaceFileCommand.newWorkspace => newWorkspace(),
        WorkspaceFileCommand.open => open(),
        WorkspaceFileCommand.recent => browseRecentFiles(),
        WorkspaceFileCommand.fromTemplate => newFromTemplate(),
        WorkspaceFileCommand.saveTemplate => saveAsTemplate(),
        WorkspaceFileCommand.save => saveWorkingFile(),
        WorkspaceFileCommand.saveAs => saveWorkingFileAs(),
        WorkspaceFileCommand.saveCopy => saveCopy(),
        WorkspaceFileCommand.export => export(),
        WorkspaceFileCommand.compatibility => compatibility(),
      };
    } finally {
      if (mounted) {
        final wasLeaving = _leavingWorkspace;
        setState(() {
          _fileCommandBusy = false;
          _leavingWorkspace = false;
        });
        if (wasLeaving) _restoreFileFocus(previousFocus);
      }
    }
  }

  void _restoreFileFocus(FocusNode? previous) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || _leavingWorkspace) return;
      if (previous?.context != null &&
          previous!.canRequestFocus &&
          (previous == _workspaceFocus ||
              previous.ancestors.contains(_workspaceFocus))) {
        previous.requestFocus();
      } else {
        _workspaceFocus.requestFocus();
      }
    });
  }

  List<WorkingFile> changedWorkingFiles({WorkingFileKind? replacing}) => [
    for (final kind in WorkingFileKind.values)
      if (replacing == null ||
          replacing == WorkingFileKind.folio ||
          kind == WorkingFileKind.folio ||
          kind == replacing)
        if (workingFiles.bound(model.workspace, kind) case final file?)
          if (file.isDirty(
            model.snapshot['fileStates'] as Map? ?? {},
            model.revision,
          ))
            file,
  ];

  Future<LeaveFilesDecision> confirmWorkingFiles({
    WorkingFileKind? replacing,
  }) async {
    while (mounted) {
      await flush();
      final files = workingFiles.pending(
        model.workspace,
        await model.currentFileStates(),
        model.revision,
        replacing: replacing,
      );
      if (files.isEmpty) return LeaveFilesDecision.proceed;
      if (!mounted) return LeaveFilesDecision.cancel;
      final choice = await showDialog<String>(
        context: context,
        barrierDismissible: false,
        builder: (context) => AlertDialog(
          title: const Text('Save changes to your files?'),
          content: SizedBox(
            width: 440,
            child: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Text(
                    'This work has changes that have not been saved to a file:',
                  ),
                  const SizedBox(height: 12),
                  for (final file in files)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 8),
                      child: Text(file.label),
                    ),
                  const Text(
                    'Save writes the changes to your files. Don’t Save discards the work you are closing; a recovery snapshot is kept on this Mac.',
                  ),
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, 'cancel'),
              child: const Text('Cancel'),
            ),
            TextButton(
              onPressed: () => Navigator.pop(context, 'discard'),
              child: const Text('Don’t Save'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, 'save'),
              child: const Text('Save'),
            ),
          ],
        ),
      );
      if (choice == 'discard') return LeaveFilesDecision.discard;
      if (choice != 'save') return LeaveFilesDecision.cancel;
      for (final pending in files) {
        final saved = pending.file == null
            ? await saveWorkingFileAs(kind: pending.kind)
            : await saveWorkingFile(target: pending.file);
        if (!saved) return LeaveFilesDecision.cancel;
      }
      // Typing during an asynchronous save remains pending. Recheck before
      // replacing the workspace or accepting a close request.
    }
    return LeaveFilesDecision.cancel;
  }

  Future<bool> prepareClose() async {
    if (_closePrepared) return true;
    if (_fileCommandBusy || !(ModalRoute.of(context)?.isCurrent ?? true)) {
      return false;
    }
    final previousFocus = FocusManager.instance.primaryFocus;
    setState(() {
      _fileCommandBusy = true;
      _leavingWorkspace = true;
    });
    try {
      await WidgetsBinding.instance.endOfFrame;
      await filesReady;
      final decision = await confirmWorkingFiles();
      if (decision == LeaveFilesDecision.cancel) return false;
      if (decision == LeaveFilesDecision.discard) {
        await preserveCurrent(reason: 'Closed without saving');
        await model.discardRecoveryOnClose();
      } else {
        await flush();
      }
      _calendarRefresh.dispose();
      agent.close();
      _closePrepared = true;
      return true;
    } finally {
      if (mounted && !_closePrepared) {
        setState(() {
          _fileCommandBusy = false;
          _leavingWorkspace = false;
        });
        _restoreFileFocus(previousFocus);
      }
    }
  }

  void _syncFileMenu() {
    final state = {
      'available':
          !_fileCommandBusy && (ModalRoute.of(context)?.isCurrent ?? true),
      'canReplace': !agent.busy,
      'spreadsheet': spreadsheet,
    };
    final encoded = jsonEncode(state);
    if (_fileMenuState == encoded || !Platform.isMacOS) return;
    _fileMenuState = encoded;
    unawaited(
      fileMenuChannel.invokeMethod<void>('state', state).catchError((
        Object error,
      ) {
        // Unit tests and other desktop hosts have no AppKit menu channel.
        if (error is! MissingPluginException && mounted) {
          notice(error.toString());
        }
      }),
    );
  }

  WorkingFile? get activeWorkingFile =>
      workingFiles.active(model.workspace, spreadsheet);

  String get fileSaveStatus {
    final file = activeWorkingFile;
    if (model.saveStatus != 'Saved on this Mac') return model.saveStatus;
    if (file == null) {
      if (workingFiles.workspaceDraft) {
        return 'Recovered copy · Not saved to a file';
      }
      return workingFiles.untitledDirty(
            model.workspace,
            model.snapshot['fileStates'] as Map? ?? {},
            spreadsheet ? WorkingFileKind.xlsx : WorkingFileKind.docx,
          )
          ? 'Saved locally · Not saved to a file'
          : model.saveStatus;
    }
    return !file.isDirty(
          model.snapshot['fileStates'] as Map? ?? {},
          model.revision,
        )
        ? 'Saved · ${file.name}'
        : 'Changes saved locally · ${file.name}';
  }

  Future<({int revision, String fingerprint, String contentSignature})>
  _writeWorkingFile(
    WorkingFileKind kind,
    String path, {
    String? expectedFingerprint,
  }) async {
    final result = await model.core.call({
      'op': kind == WorkingFileKind.folio ? 'save' : 'exportOffice',
      'path': path,
      if (kind != WorkingFileKind.folio)
        'format': ['dotx', 'xltx'].contains(path.split('.').last.toLowerCase())
            ? path.split('.').last.toLowerCase()
            : kind.name,
      'expectedFingerprint': ?expectedFingerprint,
    });
    return (
      revision: result['revision'] as int,
      fingerprint: result['fingerprint'] as String,
      contentSignature: result['contentSignature'] as String,
    );
  }

  Future<void> _rememberWorkingFile(
    WorkingFileKind kind,
    String path,
    int revision, {
    bool replaceAll = false,
    String? bookmark,
    String? fingerprint,
    required String contentSignature,
    String? previousPath,
  }) async {
    final file = WorkingFile(
      kind: kind,
      path: path,
      revision: revision,
      contentSignature: contentSignature,
      sourceKey: WorkingFiles.sourceKey(model.workspace, kind),
      bookmark: bookmark ?? await workingFiles.access.capture(path),
      fingerprint: fingerprint ?? await fileFingerprint(path),
    );
    await workingFiles.bind(
      model.workspace['id'] as String,
      file,
      replaceAll: replaceAll,
    );
    await recordRecentFile(
      kind,
      path,
      bookmark: file.bookmark,
      previousPath: previousPath,
    );
    if (['dotx', 'xltx'].contains(file.format)) {
      try {
        await templates.remember(path, previousPath: previousPath);
      } catch (e) {
        _templateError = 'The template location could not be saved: $e';
      }
    }
    if (mounted) setState(() {});
  }

  Future<bool> saveWorkingFile({WorkingFile? target}) async {
    await flush();
    final file = target ?? activeWorkingFile;
    if (file == null) return saveWorkingFileAs();
    final lease = await workingFiles.access.acquire(file);
    try {
      final fingerprint = await fileFingerprint(lease.path);
      if (fingerprint != file.fingerprint) {
        if (!mounted) return false;
        final saveAs = await showDialog<bool>(
          context: context,
          builder: (context) => AlertDialog(
            title: const Text('This file changed outside Folio'),
            content: Text(
              '${file.name} has changed since it was opened or saved. Save As keeps both versions.',
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context, false),
                child: const Text('Cancel'),
              ),
              FilledButton(
                onPressed: () => Navigator.pop(context, true),
                child: const Text('Save As…'),
              ),
            ],
          ),
        );
        return saveAs == true ? saveWorkingFileAs(target: file) : false;
      }
      final saved = await _writeWorkingFile(
        file.kind,
        lease.path,
        expectedFingerprint: file.fingerprint,
      );
      // Atomic replacement can change the file identity, so refresh its bookmark.
      await _rememberWorkingFile(
        file.kind,
        lease.path,
        saved.revision,
        fingerprint: saved.fingerprint,
        contentSignature: saved.contentSignature,
        previousPath: file.path,
      );
      notice('Saved ${File(lease.path).uri.pathSegments.last}.');
      return true;
    } finally {
      await lease.close();
    }
  }

  Future<bool> saveWorkingFileAs({
    WorkingFile? target,
    WorkingFileKind? kind,
  }) async {
    await flush();
    final current =
        target ??
        (kind == null
            ? activeWorkingFile
            : workingFiles.bound(model.workspace, kind));
    kind ??=
        current?.kind ??
        (workingFiles.workspaceDraft
            ? WorkingFileKind.folio
            : spreadsheet
            ? WorkingFileKind.xlsx
            : WorkingFileKind.docx);
    final format = current?.format ?? kind.name;
    final destination = await getSaveLocation(
      initialDirectory: current == null ? null : File(current.path).parent.path,
      suggestedName: current?.name ?? '${model.workspace['title']}.$format',
      acceptedTypeGroups: [
        XTypeGroup(
          label: switch (kind) {
            WorkingFileKind.folio => 'Folio workspace',
            WorkingFileKind.docx =>
              format == 'dotx' ? 'Word template' : 'Word document',
            WorkingFileKind.xlsx =>
              format == 'xltx' ? 'Excel template' : 'Excel workbook',
          },
          extensions: [format],
        ),
      ],
    );
    if (destination == null) return false;
    await flush();
    final path = destination.path.toLowerCase().endsWith('.$format')
        ? destination.path
        : '${destination.path}.$format';
    await workingFiles.prepareChange();
    late ({int revision, String fingerprint, String contentSignature}) saved;
    try {
      saved = await _writeWorkingFile(kind, path);
    } catch (_) {
      await workingFiles.cancelChange();
      rethrow;
    }
    await _rememberWorkingFile(
      kind,
      path,
      saved.revision,
      fingerprint: saved.fingerprint,
      contentSignature: saved.contentSignature,
    );
    notice('Saved ${File(path).uri.pathSegments.last}.');
    return true;
  }

  Future<void> saveCopy() async {
    await flush();
    final destination = await getSaveLocation(
      // NSSavePanel adds the custom file type's extension itself.
      suggestedName: Platform.isMacOS
          ? '${model.workspace['title']}'
          : '${model.workspace['title']}.folio',
      acceptedTypeGroups: const [
        XTypeGroup(label: 'Folio workspace', extensions: ['folio']),
      ],
    );
    if (destination != null) {
      await flush();
      await model.saveCopy(destination.path);
      await recordRecentFile(WorkingFileKind.folio, destination.path);
      notice('Workspace saved.');
    }
  }

  Future<void> preserveCurrent({
    String reason = 'Before opening a file',
  }) async {
    await flush();
    final path = await recoveryCopies.reserve();
    final saved = await model.core.call({
      'op': 'save',
      'path': path,
      'describe': true,
    });
    try {
      await recoveryCopies.describe(path, saved, reason);
    } catch (_) {
      // The immutable snapshot is already durable; missing preview metadata
      // must not hide or discard it, nor prevent the requested operation.
    }
  }

  Future<void> open() async {
    if (agent.busy) return;
    await flush();
    final file = await openFile(
      acceptedTypeGroups: const [
        XTypeGroup(
          label: 'Folio and Office files',
          extensions: ['folio', 'docx', 'xlsx', 'dotx', 'xltx'],
        ),
      ],
    );
    if (file == null) return;
    await openPath(file.path);
  }

  Future<bool> openPath(
    String path, {
    String? bookmark,
    String? previousPath,
  }) async {
    final resolved = await File(path).resolveSymbolicLinks();
    if (Directory(recoveryCopies.directory).existsSync()) {
      final recoveryRoot = await Directory(
        recoveryCopies.directory,
      ).resolveSymbolicLinks();
      if (resolved.startsWith('$recoveryRoot${Platform.pathSeparator}')) {
        return restoreRecovery(
          RecoveryCopy(resolved, (await File(resolved).stat()).modified),
        );
      }
    }
    final kind = workingFileKindForPath(path);
    if (!await File(path).exists()) {
      throw FileSystemException('This file no longer exists.', path);
    }
    if (await confirmWorkingFiles(replacing: kind) ==
        LeaveFilesDecision.cancel) {
      return false;
    }
    bookmark ??= await workingFiles.access.capture(path);
    final fingerprint = await fileFingerprint(path);
    await preserveCurrent();
    await workingFiles.prepareChange();
    try {
      if (kind == WorkingFileKind.folio) {
        await model.open(path);
      } else {
        await model.mutate({
          'op': 'importOffice',
          'path': path,
        }, refreshDocument: true);
        spreadsheet = kind == WorkingFileKind.xlsx;
        await model.flush();
      }
    } catch (_) {
      await workingFiles.cancelChange();
      rethrow;
    }
    await _rememberWorkingFile(
      kind,
      path,
      model.revision,
      replaceAll: kind == WorkingFileKind.folio,
      bookmark: bookmark,
      previousPath: previousPath,
      fingerprint: fingerprint,
      contentSignature: model.snapshot['fileStates'][kind.name] as String,
    );
    anchor = null;
    attachedRange = null;
    assistantUi.clear();
    agent.reset();
    setState(proposals.clear);
    return true;
  }

  Future<void> recordRecentFile(
    WorkingFileKind kind,
    String path, {
    String? bookmark,
    String? previousPath,
  }) async {
    try {
      await recentFiles.remember(
        kind,
        path,
        bookmark ?? await workingFiles.access.capture(path),
        previousPath: previousPath,
      );
    } catch (e) {
      _recentError =
          'The file operation succeeded, but its recent-file entry could not be saved: $e';
      if (mounted) notice(_recentError!);
    }
  }

  Future<void> browseRecentFiles() async {
    final choice = await showDialog<FileBrowserChoice>(
      context: context,
      builder: (_) => FileBrowser(
        recent: recentFiles,
        recovery: recoveryCopies,
        historyError: _recentError,
        inspect: (path) =>
            model.core.call({'op': 'inspectWorkspace', 'path': path}),
      ),
    );
    if (choice == null || !mounted) return;
    if (choice.browse) {
      await open();
      return;
    }
    if (choice.recovery != null) {
      await restoreRecovery(choice.recovery!);
      return;
    }
    if (choice.recent != null) await openRecentFile(choice.recent!);
  }

  Future<void> openRecentFile(RecentFile file) async {
    FileAccessLease? lease;
    try {
      lease = await workingFiles.access.acquireLocation(
        file.path,
        file.bookmark,
      );
      if (!await File(lease.path).exists()) {
        throw FileSystemException('The file was moved or removed.', lease.path);
      }
    } catch (e) {
      await lease?.close();
      if (!mounted) return;
      final locate = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: Text('Locate ${file.name}'),
          content: Text(
            'Folio could not access this saved location. Select the file again to reopen it.\n\n$e',
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('Locate file…'),
            ),
          ],
        ),
      );
      if (locate != true) return;
      final located = await openFile(
        acceptedTypeGroups: [
          XTypeGroup(
            label: file.format.toUpperCase(),
            extensions: [file.format],
          ),
        ],
      );
      if (located != null) {
        if (!located.path.toLowerCase().endsWith('.${file.format}')) {
          throw StateError('Choose a .${file.format} file.');
        }
        await openPath(located.path, previousPath: file.path);
      }
      return;
    }
    try {
      await openPath(lease.path, previousPath: file.path);
    } finally {
      await lease.close();
    }
  }

  Future<bool> restoreRecovery(RecoveryCopy copy) async {
    // Validate before asking the user to leave the current work. The final
    // recovery load validates again, and always receives a new workspace ID.
    await model.core.call({'op': 'inspectWorkspace', 'path': copy.path});
    if (await confirmWorkingFiles() == LeaveFilesDecision.cancel) return false;
    await preserveCurrent(reason: 'Before restoring a recovery copy');
    await workingFiles.prepareChange();
    try {
      await model.open(copy.path, recover: true);
    } catch (_) {
      await workingFiles.cancelChange();
      rethrow;
    }
    await workingFiles.reset(model.workspace['id'] as String, asDraft: true);
    anchor = null;
    attachedRange = null;
    assistantUi.clear();
    agent.reset();
    setState(proposals.clear);
    notice('Recovered a copy. Save chooses a new workspace file.');
    return true;
  }

  Future<void> newWorkspace() async {
    if (agent.busy) return;
    if (await confirmWorkingFiles() == LeaveFilesDecision.cancel) return;
    await preserveCurrent(reason: 'Before a new workspace');
    await workingFiles.prepareChange();
    try {
      await model.newWorkspace();
    } catch (_) {
      await workingFiles.cancelChange();
      rethrow;
    }
    await workingFiles.reset(
      model.workspace['id'] as String,
      workspace: model.workspace,
      fileStates: model.snapshot['fileStates'] as Map,
    );
    anchor = null;
    attachedRange = null;
    assistantUi.clear();
    agent.reset();
    setState(proposals.clear);
  }

  Future<Map<String, dynamic>> previewTemplate(OfficeTemplate template) async {
    final lease = await templates.acquire(template);
    try {
      return await model.core.call({
        'op': 'inspectTemplate',
        'path': lease.path,
      });
    } finally {
      await lease.close();
    }
  }

  Future<OfficeTemplate?> browseTemplate(OfficeTemplate? previous) async {
    final selected = await openFile(
      acceptedTypeGroups: const [
        XTypeGroup(
          label: 'Word and Excel templates',
          extensions: ['dotx', 'xltx'],
        ),
      ],
    );
    if (selected == null) return null;
    await model.core.call({'op': 'inspectTemplate', 'path': selected.path});
    await templates.remember(selected.path, previousPath: previous?.file?.path);
    return templates.entries.firstWhere((e) => e.file?.path == selected.path);
  }

  Future<void> newFromTemplate() async {
    final choice = await showDialog<TemplateSelection>(
      context: context,
      builder: (_) => TemplateGallery(
        library: templates,
        preview: previewTemplate,
        browse: browseTemplate,
        libraryError: _templateError,
      ),
    );
    if (choice == null || !mounted) return;
    final lease = await templates.acquire(choice.template);
    try {
      if (choice.edit) {
        await openPath(lease.path, previousPath: choice.template.file?.path);
        return;
      }
      if (await confirmWorkingFiles() == LeaveFilesDecision.cancel) return;
      await preserveCurrent(reason: 'Before creating from a template');
      await workingFiles.prepareChange();
      late Map initialFileStates;
      late String createdKind;
      try {
        await model.mutate({
          'op': 'newFromTemplate',
          'path': lease.path,
          'expectedFingerprint': choice.fingerprint,
        }, refreshDocument: true);
        initialFileStates = model.snapshot['initialFileStates'] as Map;
        createdKind = model.snapshot['newFileKind'] as String;
        await model.flush();
      } catch (_) {
        await workingFiles.cancelChange();
        rethrow;
      }
      await workingFiles.reset(
        model.workspace['id'] as String,
        workspace: model.workspace,
        fileStates: initialFileStates,
      );
      spreadsheet = createdKind == 'xlsx';
      sheetId = model.sheets.first['id'];
      anchor = null;
      attachedRange = null;
      assistantUi.clear();
      agent.reset();
      setState(proposals.clear);
      if (choice.template.file != null) {
        try {
          await templates.remember(
            lease.path,
            previousPath: choice.template.file!.path,
          );
        } catch (e) {
          notice(
            'Created the file. Its template location could not be updated: $e',
          );
        }
      }
    } finally {
      await lease.close();
    }
  }

  Future<void> saveAsTemplate() async {
    await flush();
    final format = spreadsheet ? 'xltx' : 'dotx';
    final destination = await getSaveLocation(
      suggestedName: '${model.workspace['title']}.$format',
      acceptedTypeGroups: [
        XTypeGroup(
          label: spreadsheet ? 'Excel template' : 'Word template',
          extensions: [format],
        ),
      ],
    );
    if (destination == null) return;
    await flush();
    final path = destination.path.toLowerCase().endsWith('.$format')
        ? destination.path
        : '${destination.path}.$format';
    final saved = await model.core.call({
      'op': 'exportOffice',
      'format': format,
      'path': path,
    });
    final active = activeWorkingFile;
    if (active != null &&
        File(active.path).absolute.uri.normalizePath() ==
            File(path).absolute.uri.normalizePath()) {
      await _rememberWorkingFile(
        active.kind,
        path,
        saved['revision'] as int,
        contentSignature: saved['contentSignature'] as String,
        fingerprint: saved['fingerprint'] as String,
      );
    }
    try {
      await templates.remember(path);
    } catch (e) {
      notice('Template saved. It could not be added to your library: $e');
      return;
    }
    notice('Template saved. Find it in New from template.');
  }

  Future<void> export() async {
    await flush();
    final ext = spreadsheet ? 'xlsx' : 'docx';
    final dest = await getSaveLocation(
      suggestedName: '${model.workspace['title']}.$ext',
      acceptedTypeGroups: [
        XTypeGroup(
          label: spreadsheet ? 'Excel workbook' : 'Word document',
          extensions: [ext],
        ),
      ],
    );
    if (dest != null) {
      await model.core.call({
        'op': 'exportOffice',
        'format': ext,
        'path': dest.path,
      });
      await recordRecentFile(WorkingFileKind.values.byName(ext), dest.path);
      notice(
        'Saved .$ext. Check Compatibility for supported and preserved features.',
      );
    }
  }

  void openSource(String id, String range) {
    guard(() async {
      await sheetKey.currentState?.flush();
      setState(() {
        sheetId = id;
        spreadsheet = true;
      });
      WidgetsBinding.instance.addPostFrameCallback((_) {
        final (r, c) = parseCell(range.split(':').first);
        sheetKey.currentState?.select(r, c);
      });
    });
  }

  Future<void> contextAction(String action) async {
    if (action == 'comment') {
      await showComments(add: true);
      return;
    }
    if (action == 'proofread') {
      showWritingEditor();
      return;
    }
    if (action == 'grammar') {
      await checkGrammarWithAi(false);
      return;
    }
    await flush();
    anchor = DocumentAnchor.capture(
      editor,
      model.revision,
      model.workspace['id'],
    );
    if (action == 'workbook') {
      await attachWorkbook();
      return;
    }
    setState(() {
      searchPanel = false;
      writingPanel = false;
    });
    assistantUi.showChoices(
      id: 'selection',
      title: anchor!.text.isEmpty
          ? 'Choose an action for this document'
          : 'Selected passage: “${anchor!.text.length > 160 ? '${anchor!.text.substring(0, 160)}…' : anchor!.text}”',
      choices: [
        ('Use workbook figures', 'source'),
        ('Rewrite for clarity', 'rewrite'),
        ('Summarize this passage', 'summarize'),
        ('Suggest improvements', 'review'),
        ('Check grammar with AI', 'grammar'),
      ],
    );
  }

  Future<void> attachWorkbook() async {
    await flush();
    anchor = DocumentAnchor.capture(
      editor,
      model.revision,
      model.workspace['id'],
    );
    var selected = sheetId;
    final range = TextEditingController(
      text: attachedRange?['range'] ?? (spreadsheet ? selectedCell : 'A3:D9'),
    );
    String? validation;
    Map<String, dynamic>? result;
    if (!mounted) return;
    result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, update) => AlertDialog(
          title: const Text('Use workbook data'),
          content: SizedBox(
            width: 650,
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Text(
                    'Choose the figures your assistant should use. Linked blocks update when these cells change.',
                  ),
                  const SizedBox(height: 18),
                  DropdownButtonFormField<String>(
                    initialValue: selected,
                    items: [
                      for (final sheet in model.sheets)
                        DropdownMenuItem(
                          value: sheet['id'] as String,
                          child: Text(sheet['name']),
                        ),
                    ],
                    onChanged: (value) => update(() => selected = value!),
                    decoration: const InputDecoration(
                      labelText: 'Worksheet',
                      border: OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 12),
                  draftTextField(
                    key: const Key('source-range'),
                    controller: range,
                    onChanged: (_) => update(() => validation = null),
                    decoration: InputDecoration(
                      labelText: 'Cell range',
                      hintText: 'A3:D9',
                      errorText: validation,
                      border: const OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 12),
                  WorkbookBlock(
                    model: model,
                    attributes: {
                      'sheetId': selected,
                      'range': range.text,
                      'title': 'Source preview',
                    },
                  ),
                ],
              ),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancel'),
            ),
            FilledButton(
              key: const Key('attach-range'),
              onPressed: () {
                try {
                  rangeValues({}, range.text);
                  Navigator.pop(context, {
                    'sheetId': selected,
                    'range': range.text.toUpperCase(),
                  });
                } catch (e) {
                  update(() => validation = e.toString());
                }
              },
              child: const Text('Use this range'),
            ),
          ],
        ),
      ),
    );
    range.dispose();
    if (result == null) return;
    final data = await model.core.call({'op': 'readRange', ...result});
    setState(() {
      attachedRange = data;
      searchPanel = false;
      writingPanel = false;
    });
    assistantUi.showChoices(
      id: 'workbook',
      title:
          '${data['sheetName']}!${data['range']} attached. How should we use it?',
      choices: [
        ('Write into selected passage', 'draft_from_source'),
        ('Explain these figures', 'explain_source'),
        ('Insert linked table', 'insert_table'),
        ('Insert linked chart', 'insert_chart'),
      ],
    );
  }

  Future<void> _uiAction(Map<String, dynamic> action) async {
    if (agent.busy) {
      notice('Wait for the assistant to finish, or stop it first.');
      return;
    }
    final name = action['name'];
    if (name == 'grammar') {
      await checkGrammarWithAi(false);
      return;
    }
    if (name == 'source') {
      await attachWorkbook();
      return;
    }
    if (name == 'insert_table' || name == 'insert_chart') {
      await flush();
      final source = attachedRange;
      if (source == null) throw StateError('Attach a workbook range first.');
      await _tool('propose_changes', {
        'baseRevision': model.revision,
        'summary':
            'Insert a live ${name == 'insert_chart' ? 'chart' : 'table'} from ${source['sheetName']}!${source['range']}.',
        'linkedBlock': {
          'sheetId': source['sheetId'],
          'range': source['range'],
          'title': '${source['sheetName']} · ${source['range']}',
          'mode': name == 'insert_chart' ? 'chart' : 'table',
        },
      });
      return;
    }
    prompt.text = switch (name) {
      'rewrite' =>
        'Rewrite the selected passage for clarity. Preserve the meaning. Use replaceSelection and show a proposal.',
      'summarize' =>
        'Summarize the selected passage. Use replaceSelection and show a proposal.',
      'review' =>
        'Review the selected passage and suggest improvements without changing it.',
      'draft_from_source' =>
        'Use the attached workbook range to rewrite the selected passage with accurate figures. Read the source cells, cite the worksheet and range in the text, and propose replaceSelection. If the selection is empty, append a paragraph.',
      'explain_source' =>
        'Explain the attached workbook range. Include source references and flag missing or uncertain values. Do not change anything.',
      _ =>
        'I chose an assistant control: ${jsonEncode(action)}. Read current context before any proposal.',
    };
    await send(preserveAnchor: true);
  }

  Future<void> compatibility() async {
    if (!mounted) return;
    final reports = [
      model.workspace['officeDocument']?['report'],
      model.workspace['officeWorkbook']?['report'],
    ].whereType<Map>();
    await showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Office compatibility'),
        content: SizedBox(
          width: 600,
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                const Text(
                  'DOCX and XLSX import and export are available. Full Microsoft Office fidelity is not yet complete.',
                  style: TextStyle(fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 12),
                const Text(
                  'Unchanged imported files export byte for byte. Edited files retain untouched package parts. Folio does not yet reproduce Word pagination, tracked changes, embedded objects, Excel charts, pivot tables, or every formula. Advanced content may be preserved without being displayed.',
                ),
                const SizedBox(height: 12),
                const Text(
                  'Limits: 32 MB per Office file, 30 worksheets, 100,000 populated cells per sheet. Legacy .doc/.xls, encrypted files, VBA execution and real-time coauthoring are not supported.',
                ),
                for (final report in reports) ...[
                  const Divider(),
                  Text(
                    '${report['format'].toString().toUpperCase()} · ${report['preservedParts']} package parts retained',
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                  for (final warning in report['warnings'])
                    Padding(
                      padding: const EdgeInsets.only(top: 8),
                      child: Text('• $warning'),
                    ),
                ],
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }

  Future<void> rename() async {
    var draft = model.workspace['title'] as String;
    final title = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Workspace name'),
        content: draftTextField(
          initialValue: draft,
          onChanged: (value) => draft = value,
          autofocus: true,
          maxLength: 100,
          onSubmitted: (s) => Navigator.pop(context, s),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, draft),
            child: const Text('Save'),
          ),
        ],
      ),
    );
    if (title != null && title.trim().isNotEmpty) {
      await model.mutate({'op': 'title', 'title': title.trim()});
    }
  }

  Future<void> send({bool preserveAnchor = false}) async {
    if (!preserveAnchor) reviewEntireDocument = false;
    final text = prompt.text.trim();
    if (text.isEmpty || agent.busy) return;
    await flush();
    if (!spreadsheet && !preserveAnchor) {
      anchor = DocumentAnchor.capture(
        editor,
        model.revision,
        model.workspace['id'],
      );
    }
    prompt.clear();
    await agent.send(text);
  }

  Future<void> switchMode(bool value) async {
    await sheetKey.currentState?.flush();
    if (mounted) setState(() => spreadsheet = value);
  }

  Map get currentSheet => model.sheets.firstWhere((s) => s['id'] == sheetId);

  Future<void> showSearch([bool replace = false]) async {
    final selection = editor.selection;
    final plain = editor.document.toPlainText();
    final initial =
        !spreadsheet &&
            selection.isValid &&
            !selection.isCollapsed &&
            selection.end <= plain.length
        ? plain.substring(selection.start, selection.end)
        : null;
    await flush();
    if (!mounted) return;
    setState(() {
      searchPanel = true;
      writingPanel = false;
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      searchKey.currentState?.focusQuery(replace: replace, initial: initial);
    });
  }

  Future<void> addWatch({String? range}) async {
    if (!spreadsheet || _leavingWorkspace) return;
    final scope = (model.workspace['id'], model.documentEpoch, sheetId);
    final selection =
        range ?? sheetKey.currentState?.selectedRange ?? selectedCell;
    await flush();
    if (!mounted ||
        scope != (model.workspace['id'], model.documentEpoch, sheetId)) {
      return;
    }
    final revision = model.revision;
    final added = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (_) => AddWatchDialog(
        sheets: model.sheets,
        sheetId: sheetId,
        range: selection,
        onAdd: (id, range) async {
          if (scope.$1 != model.workspace['id'] ||
              scope.$2 != model.documentEpoch ||
              revision != model.revision) {
            throw StateError(
              'The workbook changed. Close Add watch and select the cells again.',
            );
          }
          await model.mutate({
            'op': 'cellWatches',
            'workspaceId': scope.$1,
            'baseRevision': revision,
            'action': 'add',
            'sheetId': id,
            'range': range,
          });
        },
      ),
    );
    if (mounted && added == true) setState(() => watchPanel = true);
    sheetKey.currentState?.gridFocus.requestFocus();
  }

  Future<void> removeWatches(List<Map<String, dynamic>> cells) async {
    final scope = (model.workspace['id'], model.documentEpoch);
    await flush();
    if (!mounted || scope != (model.workspace['id'], model.documentEpoch)) {
      throw StateError('The workbook changed. Select the watches again.');
    }
    await model.mutate({
      'op': 'cellWatches',
      'workspaceId': scope.$1,
      'baseRevision': model.revision,
      'action': 'remove',
      'cells': cells,
    });
  }

  Future<void> openWatchedCell(
    String id,
    String address, {
    bool inspect = false,
  }) async {
    final scope = (model.workspace['id'], model.documentEpoch);
    await flush();
    if (!mounted || scope != (model.workspace['id'], model.documentEpoch)) {
      return;
    }
    if (!model.sheets.any(
      (s) => s['id'] == id && s['cellWatches']?[address] == true,
    )) {
      return;
    }
    setState(() {
      sheetId = id;
      spreadsheet = true;
    });
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted || scope != (model.workspace['id'], model.documentEpoch)) {
      return;
    }
    final (r, c) = parseCell(address);
    if (await sheetKey.currentState?.select(r, c, extend: false) == true &&
        inspect) {
      await showFormulaInspector(address);
    }
  }

  Future<void> showScenarios() async {
    if (!spreadsheet || _leavingWorkspace) return;
    final scope = (model.workspace['id'], model.documentEpoch, sheetId);
    final selection = selectedCell;
    await flush();
    if (!mounted ||
        scope != (model.workspace['id'], model.documentEpoch, sheetId)) {
      return;
    }
    var revision = model.revision;
    bool current() =>
        mounted &&
        !_leavingWorkspace &&
        scope.$1 == model.workspace['id'] &&
        scope.$2 == model.documentEpoch &&
        revision == model.revision;
    final result = await showDialog<String>(
      context: context,
      builder: (_) => ScenarioManager(
        sheets: () => model.sheets,
        sheetId: scope.$3,
        selection: selection,
        changes: model,
        isCurrent: current,
        onBrowseSource: () async {
          final file = await openFile(
            acceptedTypeGroups: const [
              XTypeGroup(
                label: 'Excel workbooks',
                extensions: ['xlsx', 'xltx'],
              ),
            ],
          );
          if (file == null) return null;
          if (!current()) {
            throw StateError('The workbook changed. Reopen Scenario Manager.');
          }
          final source = await model.core.call({
            'op': 'inspectScenarioSource',
            'path': file.path,
            'sheetId': scope.$3,
            'workspaceId': scope.$1,
            'baseRevision': revision,
          });
          return {...source, 'name': file.name};
        },
        onRead: (request) async {
          if (!current()) {
            throw StateError('The workbook changed. Reopen Scenario Manager.');
          }
          return model.core.call({
            ...request,
            'workspaceId': scope.$1,
            'baseRevision': revision,
          });
        },
        onCommand: (request) async {
          if (!current()) {
            throw StateError('The workbook changed. Reopen Scenario Manager.');
          }
          await model.mutate({
            ...request,
            'workspaceId': scope.$1,
            'baseRevision': revision,
          });
          if (!mounted ||
              scope.$1 != model.workspace['id'] ||
              scope.$2 != model.documentEpoch) {
            throw StateError('The active workbook changed.');
          }
          revision = model.revision;
        },
      ),
    );
    if (!mounted) return;
    if (result == 'summary' && current()) {
      setState(() {
        sheetId = model.sheets.last['id'] as String;
        selectedCell = 'A1';
      });
    }
    sheetKey.currentState?.gridFocus.requestFocus();
  }

  Future<void> showGoalSeek() async {
    if (!spreadsheet || _leavingWorkspace) return;
    final scope = (model.workspace['id'], model.documentEpoch, sheetId);
    final address =
        sheetKey.currentState?.selected ?? selectedCell.split(':').first;
    await flush();
    if (!mounted ||
        scope != (model.workspace['id'], model.documentEpoch, sheetId)) {
      return;
    }
    final revision = model.revision,
        generation = model.snapshot['evaluatedCells'];
    bool current() =>
        mounted &&
        !_leavingWorkspace &&
        scope.$1 == model.workspace['id'] &&
        scope.$2 == model.documentEpoch &&
        revision == model.revision &&
        generation == model.snapshot['evaluatedCells'];
    await showDialog<bool>(
      context: context,
      builder: (_) => GoalSeekDialog(
        sheets: model.sheets,
        sheetId: sheetId,
        address: address,
        changes: model,
        isCurrent: current,
        onCall: (request) async {
          if (request['op'] != 'goalSeekCancel' && !current()) {
            throw StateError('The workbook changed. Reopen Goal Seek.');
          }
          return model.core.call({
            'workspaceId': scope.$1,
            'baseRevision': revision,
            'calculationGeneration': generation,
            ...request,
          });
        },
        onApply: (request) async {
          if (!current()) {
            throw StateError('The workbook changed. Run Goal Seek again.');
          }
          await model.mutate(request);
        },
      ),
    );
    if (mounted) sheetKey.currentState?.gridFocus.requestFocus();
  }

  Future<void> showFormulaInspector(String address) async {
    if (!spreadsheet || _leavingWorkspace) return;
    final requestedSheet = sheetId, requestedEpoch = model.documentEpoch;
    await flush();
    if (!mounted ||
        requestedSheet != sheetId ||
        requestedEpoch != model.documentEpoch) {
      return;
    }
    final workspaceId = model.workspace['id'], revision = model.revision;
    final generation = model.snapshot['evaluatedCells'];
    bool isCurrent() =>
        mounted &&
        !_leavingWorkspace &&
        model.workspace['id'] == workspaceId &&
        model.revision == revision &&
        model.documentEpoch == requestedEpoch &&
        model.snapshot['evaluatedCells'] == generation;
    void checkCurrent() {
      if (!isCurrent()) {
        throw StateError('The workbook changed. Reopen Formula inspector.');
      }
    }

    final target = await showDialog<Map>(
      context: context,
      builder: (_) => FormulaInspector(
        sheetId: requestedSheet,
        address: address,
        changes: model,
        isCurrent: isCurrent,
        onRead: (request) async {
          checkCurrent();
          final result = await model.core.call({
            'op': 'formulaAudit',
            'workspaceId': workspaceId,
            'baseRevision': revision,
            'calculationGeneration': generation,
            ...request,
          });
          checkCurrent();
          return result;
        },
      ),
    );
    if (!mounted) return;
    if (target == null) {
      sheetKey.currentState?.gridFocus.requestFocus();
      return;
    }
    checkCurrent();
    setState(() => sheetId = target['sheetId']);
    await WidgetsBinding.instance.endOfFrame;
    checkCurrent();
    final range = (target['range'] as String).split(':');
    final (r1, c1) = parseCell(range.first);
    final (r2, c2) = parseCell(range.last);
    final view = sheetKey.currentState;
    if (await view?.select(r1, c1, extend: false) == true && range.length > 1) {
      checkCurrent();
      await view?.select(r2, c2, extend: true, shiftPressed: true);
    }
  }

  Future<void> showSheetReviews({String? kind, String? address}) async {
    if (!spreadsheet || _leavingWorkspace) return;
    await flush();
    if (!mounted) return;
    final workspaceId = model.workspace['id'];
    var revision = model.revision;
    final targetSheetId = sheetId;
    final targetAddress = address ?? selectedCell.split(':').first;
    void checkCurrent() {
      if (model.workspace['id'] != workspaceId || model.revision != revision) {
        throw StateError(
          'The workbook changed. Close Comments & notes and reopen it.',
        );
      }
    }

    final target = await showDialog<Map>(
      context: context,
      barrierDismissible: false,
      builder: (_) => SheetReviewsDialog(
        workbook: model.workspace,
        sheetId: targetSheetId,
        address: targetAddress,
        newKind: kind,
        onChange: (request) async {
          checkCurrent();
          await model.mutate({
            'op': 'sheetReview',
            'workspaceId': workspaceId,
            'baseRevision': revision,
            ...request,
          });
          revision = model.revision;
          return model.workspace;
        },
        onHistory: (redo) async {
          checkCurrent();
          await model.mutate({'op': redo ? 'redo' : 'undo'});
          revision = model.revision;
          return model.workspace;
        },
      ),
    );
    if (!mounted) return;
    if (target != null) {
      checkCurrent();
      setState(() => sheetId = target['sheetId']);
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted) return;
      final (r, c) = parseCell(target['address']);
      await sheetKey.currentState?.select(r, c, extend: false);
    } else {
      sheetKey.currentState?.gridFocus.requestFocus();
    }
  }

  Future<void> discardUnrecordedEdits() async {
    if (_documentError == null || _discardingDocumentEdits) return;
    setState(() => _discardingDocumentEdits = true);
    editor.readOnly = true;
    try {
      await _documentPending.catchError((Object _) {});
      await _historyPending.catchError((Object _) {});
      _historyInput = null;
      _rejectedInput = null;
      _inputChange = null;
      _documentPending = Future.value();
      _historyPending = Future.value();
      _documentEditSerial++;
      _documentError = null;
      await model.restoreCommittedDocument();
    } finally {
      if (mounted) setState(() => _discardingDocumentEdits = false);
      editor.readOnly = recordingNeedsAuthor;
    }
  }

  Future<void> showTrackingSettings({bool? enabled}) async {
    if (spreadsheet ||
        _leavingWorkspace ||
        _trackingBusy ||
        ModalRoute.of(context)?.isCurrent == false) {
      return;
    }
    _trackingBusy = true;
    try {
      await flush();
      final data = await model.core.call({'op': 'documentRevisions'});
      if (!mounted) return;
      final settings = Map<String, dynamic>.from(data['tracking'] as Map);
      final first = (settings['author'] as String).isEmpty;
      if (first) {
        settings['formatting'] = data['sourceRecording']?['formatting'] ?? true;
      }
      settings['enabled'] = enabled ?? (first ? true : settings['enabled']);
      await showDialog<void>(
        context: context,
        barrierDismissible: false,
        builder: (_) => DocumentTrackingDialog(
          settings: settings,
          unavailableReason: data['protected'] == true
              ? 'This document has enforced editing protection. Change its protection in Word first.'
              : data['recordingReason'] as String?,
          onApply: (request) async {
            await model.mutate({
              'op': 'documentTracking',
              'workspaceId': data['workspaceId'],
              'baseRevision': data['baseRevision'],
              ...request,
            }, refreshDocument: true);
          },
        ),
      );
    } finally {
      _trackingBusy = false;
    }
  }

  Future<void> toggleTracking() async {
    if (spreadsheet ||
        _leavingWorkspace ||
        _trackingBusy ||
        ModalRoute.of(context)?.isCurrent == false) {
      return;
    }
    if (recording == null) return showTrackingSettings(enabled: true);
    _trackingBusy = true;
    try {
      await flush();
      await model.mutate({
        'op': 'documentTracking',
        'workspaceId': model.workspace['id'],
        'baseRevision': model.revision,
        'author': recording!['author'],
        'enabled': recording!['enabled'] != true,
        'formatting': recording!['formatting'],
      }, refreshDocument: true);
    } finally {
      _trackingBusy = false;
    }
  }

  Future<void> showPageSetup({bool columns = false}) async {
    if (spreadsheet || _leavingWorkspace) return;
    await flush();
    final selection = editor.selection;
    final data = await model.core.call({'op': 'documentPageSetup'});
    if (!mounted) return;
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => columns
          ? DocumentColumnsDialog(
              data: data,
              initialSection: pageSectionAtSelection(
                model.workspace['document'] as Map,
                selection,
              ),
            )
          : DocumentPageSetupDialog(
              data: data,
              initialSection: pageSectionAtSelection(
                model.workspace['document'] as Map,
                selection,
              ),
            ),
    );
    if (result == null || !mounted) return;
    await flush();
    await model.mutate({
      'op': 'documentPageSetupApply',
      ...result,
      'workspaceId': data['workspaceId'],
      'baseRevision': data['baseRevision'],
    }, refreshDocument: true);
    if (mounted && selection.end < editor.document.length) {
      editor.updateSelection(selection, q.ChangeSource.local);
    }
  }

  Future<void> showPagePreview() async {
    if (spreadsheet || _leavingWorkspace) return;
    await flush();
    final snapshot = await model.core.call({'op': 'documentPagePreview'});
    if (!mounted) return;
    await showDialog<void>(
      context: context,
      builder: (_) => DocumentPagePreview(snapshot: snapshot),
    );
  }

  Future<void> showHeaders() async {
    if (spreadsheet || _leavingWorkspace) return;
    await flush();
    if (!mounted) return;
    await showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (_) => DocumentHeadersDialog(
        initialSection: pageSectionAtSelection(
          model.workspace['document'] as Map,
          editor.selection,
        ),
        read: (target) async {
          await flush();
          return model.core.call({'op': 'documentHeaders', ...target});
        },
        apply: (request) async {
          await flush();
          await model.mutate({
            'op': 'documentHeadersApply',
            ...request,
          }, refreshDocument: true);
        },
      ),
    );
  }

  Future<void> showSectionBreaks() async {
    if (spreadsheet || _leavingWorkspace) return;
    await flush();
    final selection = editor.selection;
    final index = DocumentBookmarkIndex(model.workspace['document'] as Map);
    final logical = selection.isValid && selection.isCollapsed
        ? index.logicalOffset(selection.start)
        : null;
    final paragraph = logical == null ? null : index.paragraph(logical);
    final target = <String, dynamic>{
      if (paragraph != null && !paragraph.embedded) ...{
        'path': paragraph.path,
        'offset': logical! - paragraph.start,
      },
    };
    final data = await model.core.call({
      'op': 'documentSectionContext',
      ...target,
    });
    if (_reviewingDocument) {
      data['canInsert'] = false;
      data['insertReason'] =
          'Return to editing to place an insertion point for a new section.';
    }
    if (!mounted) return;
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (_) => DocumentSectionBreakDialog(data: data),
    );
    if (result == null || !mounted) return;
    await flush();
    await model.mutate({
      'op': 'documentSectionBreak',
      ...target,
      ...result,
      'workspaceId': data['workspaceId'],
      'baseRevision': data['baseRevision'],
    }, refreshDocument: true);
    if (mounted && result['action'] == 'insert') {
      editor.updateSelection(
        TextSelection.collapsed(
          offset: (selection.start + 1).clamp(0, editor.document.length - 1),
        ),
        q.ChangeSource.local,
      );
    }
  }

  Future<void> showDocumentReview() async {
    if (spreadsheet || _leavingWorkspace || _reviewingDocument) return;
    await flush();
    if (!mounted) return;
    FocusScope.of(context).unfocus();
    editor.readOnly = true;
    setState(() => _reviewingDocument = true);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && _reviewingDocument) _workspaceFocus.requestFocus();
    });
  }

  Future<Map> readDocumentReview(String mode) async {
    await flush();
    final result = await model.core.call({
      'op': 'documentReviewView',
      'mode': mode,
    });
    if (result['workspaceId'] != model.workspace['id'] ||
        result['baseRevision'] != model.revision) {
      throw StateError(
        'The document changed while preparing this view. Retry to refresh it.',
      );
    }
    return result;
  }

  Future<void> showRevisions() async {
    if (spreadsheet || _leavingWorkspace) return;
    await flush();
    final data = await model.core.call({'op': 'documentRevisions'});
    if (!mounted) return;
    final workspaceId = data['workspaceId'];
    var revision = data['baseRevision'];
    void checkCurrent() {
      if (model.workspace['id'] != workspaceId || model.revision != revision) {
        throw StateError(
          'The document changed. Close Track Changes and reopen it.',
        );
      }
    }

    Future<Map> read() async {
      revision = model.revision;
      return model.core.call({'op': 'documentRevisions'});
    }

    await showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (_) => DocumentRevisionsDialog(
        data: data,
        onChange: (request) async {
          checkCurrent();
          await model.mutate({
            'op': 'documentRevision',
            'workspaceId': workspaceId,
            'baseRevision': revision,
            ...request,
          }, refreshDocument: true);
          return read();
        },
        onHistory: (redo) async {
          checkCurrent();
          await model.mutate({
            'op': redo ? 'redo' : 'undo',
          }, refreshDocument: true);
          return read();
        },
      ),
    );
  }

  Future<void> showComments({bool add = false}) async {
    if (spreadsheet || _leavingWorkspace) return;
    await flush();
    if (!mounted) return;
    final workspaceId = model.workspace['id'];
    var revision = model.revision;
    final document = model.workspace['document'] as Map;
    final selection = DocumentBookmarkIndex(document).capture(editor.selection);
    void checkCurrent() {
      if (model.workspace['id'] != workspaceId || model.revision != revision) {
        throw StateError('The document changed. Close Comments and reopen it.');
      }
    }

    final id = await showDialog<String>(
      context: context,
      barrierDismissible: false,
      builder: (_) => DocumentCommentsDialog(
        document: document,
        selection: selection,
        newComment: add,
        onChange: (action) async {
          checkCurrent();
          await model.mutate({
            'op': 'documentComment',
            'workspaceId': workspaceId,
            'baseRevision': revision,
            ...action,
          });
          revision = model.revision;
          return model.workspace['document'];
        },
        onHistory: (redo) async {
          checkCurrent();
          await model.mutate({'op': redo ? 'redo' : 'undo'});
          revision = model.revision;
          return model.workspace['document'];
        },
      ),
    );
    if (!mounted || id == null) return;
    checkCurrent();
    final thread = documentComments(
      model.workspace['document'],
    ).where((t) => t['id'] == id).firstOrNull;
    if (thread == null) throw StateError('This comment no longer exists.');
    await documentKey.currentState?.revealBookmark(
      model.workspace['document'],
      thread,
    );
  }

  Future<void> showBookmarks() async {
    if (spreadsheet) return;
    await flush();
    if (!mounted) return;
    final selection = editor.selection;
    final workspaceId = model.workspace['id'];
    var revision = model.revision;
    final document = model.workspace['document'] as Map;
    final selected = DocumentBookmarkIndex(document).capture(selection);
    final name = await showDialog<String>(
      context: context,
      builder: (_) => DocumentBookmarksDialog(
        document: document,
        selection: selected,
        onChange: (action) async {
          if (model.workspace['id'] != workspaceId ||
              model.revision != revision) {
            throw StateError(
              'The document changed. Close Bookmarks and reopen it.',
            );
          }
          await model.mutate({
            'op': 'bookmark',
            'baseRevision': revision,
            'workspaceId': workspaceId,
            ...action,
          });
          revision = model.revision;
          return model.workspace['document'];
        },
      ),
    );
    if (!mounted || name == null) return;
    if (model.workspace['id'] != workspaceId || model.revision != revision) {
      throw StateError(
        'The document changed. Reopen Bookmarks to go to its current location.',
      );
    }
    final bookmark = documentBookmarks(
      model.workspace['document'],
    ).where((b) => b['name'] == name).firstOrNull;
    if (bookmark == null) throw StateError('This bookmark no longer exists.');
    await documentKey.currentState?.revealBookmark(
      model.workspace['document'],
      bookmark,
    );
  }

  Future<void> navigateSearch(SearchHit hit) async {
    if (hit.block != null) {
      documentKey.currentState?.reveal(hit.documentStart, hit.documentEnd);
    } else {
      await sheetKey.currentState?.flush();
      if (!mounted) return;
      setState(() => sheetId = hit.sheetId!);
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted) return;
      final (row, col) = parseCell(hit.address!);
      await sheetKey.currentState?.select(
        row,
        col,
        requestFocus: false,
        extend: false,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    model.rowTextScaler = MediaQuery.textScalerOf(context);
    model.rowTextDirection = Directionality.of(context);
    _historyRouteCurrent = ModalRoute.of(context)?.isCurrent ?? true;
    _syncFileMenu();
    _syncHistoryMenu();
    return Shortcuts(
      shortcuts: {
        ...workspaceFileShortcuts(reserveControlShiftE: !spreadsheet),
        if (!spreadsheet) trackingShortcut: const ToggleTrackingIntent(),
        ...searchShortcuts,
        if (!spreadsheet) ...bookmarkShortcuts,
        ...commentShortcuts,
        if (spreadsheet)
          const SingleActivator(LogicalKeyboardKey.f2, shift: true):
              const NewNoteIntent(),
        if (spreadsheet)
          const SingleActivator(LogicalKeyboardKey.f9):
              const RecalculateIntent(),
        const SingleActivator(LogicalKeyboardKey.keyZ, meta: true):
            const UndoWorkspace(),
        const SingleActivator(LogicalKeyboardKey.keyZ, meta: true, shift: true):
            const UndoWorkspace(true),
      },
      child: Actions(
        actions: {
          ToggleTrackingIntent: CallbackAction<ToggleTrackingIntent>(
            onInvoke: (_) {
              guard(toggleTracking);
              return null;
            },
          ),
          RecalculateIntent: CallbackAction<RecalculateIntent>(
            onInvoke: (_) {
              if (_leavingWorkspace) return null;
              guard(recalculate);
              return null;
            },
          ),
          NewCommentIntent: CallbackAction<NewCommentIntent>(
            onInvoke: (_) {
              if (!_leavingWorkspace) {
                guard(
                  () => spreadsheet
                      ? showSheetReviews(kind: 'comment')
                      : showComments(add: true),
                );
              }
              return null;
            },
          ),
          NewNoteIntent: CallbackAction<NewNoteIntent>(
            onInvoke: (_) {
              if (!_leavingWorkspace) {
                guard(() => showSheetReviews(kind: 'note'));
              }
              return null;
            },
          ),
          BookmarksIntent: CallbackAction<BookmarksIntent>(
            onInvoke: (_) {
              if (_leavingWorkspace) return null;
              guard(showBookmarks);
              return null;
            },
          ),
          FindIntent: CallbackAction<FindIntent>(
            onInvoke: (intent) {
              if (_leavingWorkspace) return null;
              guard(() => showSearch(intent.replace));
              return null;
            },
          ),
          FindNextIntent: CallbackAction<FindNextIntent>(
            onInvoke: (intent) {
              if (_leavingWorkspace) return null;
              if (searchPanel) {
                searchKey.currentState?.next(intent.backwards);
              } else {
                guard(showSearch);
              }
              return null;
            },
          ),
          WorkspaceFileIntent: CallbackAction<WorkspaceFileIntent>(
            onInvoke: (intent) {
              if (_leavingWorkspace) return null;
              guard(() => fileCommand(intent.command));
              return null;
            },
          ),
          UndoWorkspace: CallbackAction<UndoWorkspace>(
            onInvoke: (intent) {
              if (_leavingWorkspace) return null;
              historyShortcut(intent.redo);
              return null;
            },
          ),
        },
        child: FocusScope(
          node: _workspaceFocus,
          autofocus: true,
          canRequestFocus: !_leavingWorkspace,
          descendantsAreFocusable: !_leavingWorkspace,
          child: AbsorbPointer(
            absorbing: _leavingWorkspace,
            child: Scaffold(
              body: Column(
                children: [
                  Container(
                    height: 52,
                    color: officeColor,
                    padding: const EdgeInsets.only(left: 86, right: 14),
                    child: Row(
                      children: [
                        const Text(
                          'FOLIO',
                          style: TextStyle(
                            color: Colors.white,
                            fontSize: 14,
                            fontWeight: FontWeight.w700,
                            letterSpacing: 1,
                          ),
                        ),
                        const SizedBox(width: 18),
                        if (spreadsheet)
                          IconButton(
                            key: const Key('calculate-now'),
                            tooltip: 'Recalculate workbook (F9)',
                            onPressed: () => guard(recalculate),
                            icon: const Icon(
                              Icons.calculate_outlined,
                              color: Colors.white,
                              size: 19,
                            ),
                          ),
                        IconButton(
                          tooltip: 'Save (⌘S / Ctrl+S)',
                          onPressed:
                              fileCommandEnabled(WorkspaceFileCommand.save)
                              ? () => guard(
                                  () => fileCommand(WorkspaceFileCommand.save),
                                )
                              : null,
                          icon: const Icon(
                            Icons.save_outlined,
                            color: Colors.white,
                            size: 19,
                          ),
                        ),
                        IconButton(
                          key: const Key('undo'),
                          tooltip: 'Undo',
                          onPressed: model.canUndo
                              ? () => guard(() => history(false))
                              : null,
                          icon: const Icon(
                            Icons.undo,
                            color: Colors.white,
                            size: 19,
                          ),
                        ),
                        IconButton(
                          key: const Key('redo'),
                          tooltip: 'Redo',
                          onPressed: model.canRedo
                              ? () => guard(() => history(true))
                              : null,
                          icon: const Icon(
                            Icons.redo,
                            color: Colors.white,
                            size: 19,
                          ),
                        ),
                        const SizedBox(width: 15),
                        Expanded(
                          child: GestureDetector(
                            onTap: () => guard(rename),
                            child: Text(
                              '${model.workspace['title']}  ·  ${spreadsheet ? 'Workbook' : 'Document'}',
                              style: const TextStyle(
                                color: Colors.white,
                                fontSize: 13,
                              ),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ),
                        ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 260),
                          child: Tooltip(
                            message: fileSaveStatus,
                            child: Text(
                              fileSaveStatus,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: const TextStyle(
                                color: Colors.white70,
                                fontSize: 11,
                              ),
                            ),
                          ),
                        ),
                        const SizedBox(width: 16),
                        TextButton(
                          onPressed:
                              fileCommandEnabled(WorkspaceFileCommand.export)
                              ? () => guard(
                                  () =>
                                      fileCommand(WorkspaceFileCommand.export),
                                )
                              : null,
                          child: Text(
                            spreadsheet ? 'Export Excel' : 'Export Word',
                            style: const TextStyle(color: Colors.white),
                          ),
                        ),
                      ],
                    ),
                  ),
                  Expanded(
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        _sidebar(),
                        Expanded(child: _workspace()),
                        SizedBox(
                          width: 360,
                          child: searchPanel
                              ? SearchPanel(
                                  key: searchKey,
                                  model: model,
                                  editor: editor,
                                  spreadsheet: spreadsheet,
                                  sheetId: sheetId,
                                  flush: flush,
                                  onNavigate: navigateSearch,
                                  onClose: () =>
                                      setState(() => searchPanel = false),
                                )
                              : writingPanel
                              ? WritingPanel(
                                  controller: proofreading,
                                  onClose: () =>
                                      setState(() => writingPanel = false),
                                  onSelect: (issue) => editor.updateSelection(
                                    TextSelection(
                                      baseOffset: issue.start,
                                      extentOffset: issue.end,
                                    ),
                                    q.ChangeSource.local,
                                  ),
                                  onFix: (issue, replacement) => guard(
                                    () => fixWriting(issue, replacement),
                                  ),
                                  onAi: (entire) =>
                                      guard(() => checkGrammarWithAi(entire)),
                                  guard: guard,
                                  aiBusy: agent.busy,
                                )
                              : _assistant(),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _badge(String s) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 4),
    decoration: BoxDecoration(
      border: Border.all(color: line),
      borderRadius: BorderRadius.circular(4),
    ),
    child: Text(
      s,
      style: const TextStyle(fontSize: 8, letterSpacing: 1.3, color: muted),
    ),
  );
  Widget _sidebar() => Container(
    width: 62,
    decoration: const BoxDecoration(
      color: Color(0xfff5f6f8),
      border: Border(right: BorderSide(color: line)),
    ),
    child: Column(
      children: [
        const SizedBox(height: 12),
        IconButton(
          tooltip: 'Document',
          isSelected: !spreadsheet,
          onPressed: () => guard(() => switchMode(false)),
          icon: const Icon(Icons.description_outlined),
          selectedIcon: const Icon(Icons.description, color: Color(0xff185abd)),
        ),
        const Text('Word', style: TextStyle(fontSize: 9)),
        const SizedBox(height: 18),
        IconButton(
          tooltip: 'Workbook',
          isSelected: spreadsheet,
          onPressed: () => guard(() => switchMode(true)),
          icon: const Icon(Icons.grid_on),
          selectedIcon: const Icon(Icons.grid_on, color: Color(0xff107c41)),
        ),
        const Text('Sheets', style: TextStyle(fontSize: 9)),
        const SizedBox(height: 18),
        IconButton(
          tooltip: 'Open workspace or Office file (⌘O / Ctrl+O)',
          onPressed: fileCommandEnabled(WorkspaceFileCommand.open)
              ? () => guard(() => fileCommand(WorkspaceFileCommand.open))
              : null,
          icon: const Icon(Icons.folder_open),
        ),
        IconButton(
          tooltip: 'New workspace (⌘N / Ctrl+N)',
          onPressed: fileCommandEnabled(WorkspaceFileCommand.newWorkspace)
              ? () =>
                    guard(() => fileCommand(WorkspaceFileCommand.newWorkspace))
              : null,
          icon: const Icon(Icons.add),
        ),
        IconButton(
          tooltip: 'Recent files and recovery',
          onPressed: fileCommandEnabled(WorkspaceFileCommand.recent)
              ? () => guard(() => fileCommand(WorkspaceFileCommand.recent))
              : null,
          icon: const Icon(Icons.history),
        ),
        IconButton(
          tooltip: 'New from template',
          icon: const Icon(Icons.dashboard_customize_outlined),
          onPressed: fileCommandEnabled(WorkspaceFileCommand.fromTemplate)
              ? () =>
                    guard(() => fileCommand(WorkspaceFileCommand.fromTemplate))
              : null,
        ),
        const Spacer(),
        IconButton(
          tooltip: 'Office compatibility',
          onPressed: () =>
              guard(() => fileCommand(WorkspaceFileCommand.compatibility)),
          icon: const Icon(Icons.verified_outlined),
        ),
        const SizedBox(height: 12),
      ],
    ),
  );
  Widget _workspace() => Column(
    children: [
      if (model.snapshot['calculationError'] != null)
        MaterialBanner(
          content: Text(
            'Some calculations could not be completed: ${model.snapshot['calculationError']}',
          ),
          actions: [
            TextButton(
              onPressed: () =>
                  guard(() => fileCommand(WorkspaceFileCommand.compatibility)),
              child: const Text('Details'),
            ),
          ],
        ),
      if (model.error != null)
        Container(
          color: const Color(0xffffeed8),
          padding: const EdgeInsets.all(12),
          child: SelectableText(
            model.error!,
            style: const TextStyle(fontSize: 11),
          ),
        ),
      Container(
        height: 40,
        color: const Color(0xfff4f5f7),
        padding: const EdgeInsets.symmetric(horizontal: 10),
        child: Row(
          children: [
            Expanded(
              child: SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: Row(
                  children: [
                    PopupMenuButton<WorkspaceFileCommand>(
                      tooltip: 'File',
                      enabled: !_fileCommandBusy,
                      onSelected: (command) =>
                          guard(() => fileCommand(command)),
                      itemBuilder: (_) => [
                        PopupMenuItem(
                          value: WorkspaceFileCommand.newWorkspace,
                          enabled: !agent.busy,
                          child: const Text('New workspace'),
                        ),
                        PopupMenuItem(
                          value: WorkspaceFileCommand.fromTemplate,
                          enabled: !agent.busy,
                          child: const Text('New from template…'),
                        ),
                        PopupMenuItem(
                          value: WorkspaceFileCommand.open,
                          enabled: !agent.busy,
                          child: const Text('Open workspace or Office file…'),
                        ),
                        PopupMenuItem(
                          value: WorkspaceFileCommand.recent,
                          enabled: !agent.busy,
                          child: const Text('Recent files and recovery…'),
                        ),
                        const PopupMenuDivider(),
                        const PopupMenuItem(
                          value: WorkspaceFileCommand.save,
                          child: Text('Save'),
                        ),
                        const PopupMenuItem(
                          value: WorkspaceFileCommand.saveAs,
                          child: Text('Save As…'),
                        ),
                        const PopupMenuItem(
                          value: WorkspaceFileCommand.saveCopy,
                          child: Text('Save workspace copy…'),
                        ),
                        const PopupMenuItem(
                          value: WorkspaceFileCommand.saveTemplate,
                          child: Text('Save as template…'),
                        ),
                        PopupMenuItem(
                          value: WorkspaceFileCommand.export,
                          child: Text(
                            spreadsheet ? 'Export Excel…' : 'Export Word…',
                          ),
                        ),
                        const PopupMenuDivider(),
                        const PopupMenuItem(
                          value: WorkspaceFileCommand.compatibility,
                          child: Text('Compatibility report'),
                        ),
                      ],
                      child: const Padding(
                        padding: EdgeInsets.symmetric(horizontal: 14),
                        child: Text('File'),
                      ),
                    ),
                    for (final tab in [
                      'Home',
                      'Insert',
                      if (spreadsheet) 'Formulas',
                      if (spreadsheet) 'Data',
                      if (!spreadsheet) 'Layout',
                      'Review',
                    ])
                      TextButton(
                        onPressed: () => setState(() => ribbon = tab),
                        style: TextButton.styleFrom(
                          foregroundColor: ribbon == tab ? officeColor : ink,
                        ),
                        child: Text(
                          tab,
                          style: TextStyle(
                            fontWeight: ribbon == tab
                                ? FontWeight.w600
                                : FontWeight.normal,
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ),
            IconButton(
              key: const Key('find-replace'),
              tooltip: 'Find and replace',
              onPressed: () => guard(showSearch),
              icon: const Icon(Icons.search, size: 20),
            ),
            if (!spreadsheet)
              IconButton(
                key: const Key('writing-editor'),
                tooltip: 'Spelling and grammar',
                onPressed: showWritingEditor,
                icon: Badge(
                  isLabelVisible: proofreading.issues.isNotEmpty,
                  label: Text('${proofreading.issues.length}'),
                  child: const Icon(Icons.spellcheck, size: 20),
                ),
              ),
            TextButton.icon(
              onPressed: () => guard(attachWorkbook),
              icon: const Icon(Icons.attach_file, size: 16),
              label: const Text('Use workbook'),
            ),
          ],
        ),
      ),
      if (ribbon == 'Layout' && !spreadsheet)
        Container(
          color: Colors.white,
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
          child: Wrap(
            crossAxisAlignment: WrapCrossAlignment.center,
            spacing: 6,
            children: [
              TextButton.icon(
                key: const Key('document-page-setup'),
                onPressed: () => guard(showPageSetup),
                icon: const Icon(Icons.description_outlined, size: 18),
                label: const Text('Page setup…'),
              ),
              TextButton.icon(
                key: const Key('document-section-breaks'),
                onPressed: () => guard(showSectionBreaks),
                icon: const Icon(Icons.splitscreen_outlined, size: 18),
                label: const Text('Section breaks…'),
              ),
              TextButton.icon(
                key: const Key('document-columns'),
                onPressed: () => guard(() => showPageSetup(columns: true)),
                icon: const Icon(Icons.view_column_outlined, size: 18),
                label: const Text('Columns…'),
              ),
              TextButton.icon(
                key: const Key('document-headers'),
                onPressed: () => guard(showHeaders),
                icon: const Icon(Icons.vertical_distribute, size: 18),
                label: const Text('Headers & footers…'),
              ),
              TextButton.icon(
                key: const Key('document-page-preview'),
                onPressed: () => guard(showPagePreview),
                icon: const Icon(Icons.find_in_page_outlined, size: 18),
                label: const Text('Page preview…'),
              ),
              const SizedBox(width: 12),
              const Text(
                'Continuous view · Section 1 page size',
                style: TextStyle(fontSize: 12, color: muted),
              ),
            ],
          ),
        ),
      if (ribbon == 'Data' && spreadsheet)
        Container(
          color: Colors.white,
          padding: const EdgeInsets.all(8),
          child: Wrap(
            spacing: 8,
            children: [
              TextButton.icon(
                key: const Key('scenario-manager-button'),
                onPressed: () => guard(showScenarios),
                icon: const Icon(Icons.layers_outlined, size: 18),
                label: const Text('Scenario Manager'),
              ),
              TextButton.icon(
                key: const Key('goal-seek-button'),
                onPressed: () => guard(showGoalSeek),
                icon: const Icon(Icons.adjust, size: 18),
                label: const Text('Goal Seek'),
              ),
            ],
          ),
        ),
      if (ribbon == 'Formulas' && spreadsheet)
        Container(
          color: Colors.white,
          padding: const EdgeInsets.all(8),
          child: Wrap(
            spacing: 8,
            children: [
              TextButton.icon(
                key: const Key('formula-inspector-button'),
                onPressed: () => guard(
                  () => showFormulaInspector(
                    sheetKey.currentState?.selected ??
                        selectedCell.split(':').first,
                  ),
                ),
                icon: const Icon(Icons.account_tree_outlined, size: 18),
                label: const Text('Formula inspector'),
              ),
              TextButton.icon(
                key: const Key('watch-toggle'),
                onPressed: () => setState(() => watchPanel = !watchPanel),
                icon: Icon(
                  watchPanel ? Icons.visibility : Icons.visibility_outlined,
                  size: 18,
                ),
                label: const Text('Watch Window'),
              ),
              TextButton.icon(
                onPressed: () => guard(recalculate),
                icon: const Icon(Icons.calculate_outlined, size: 18),
                label: const Text('Calculate now'),
              ),
            ],
          ),
        ),
      if (ribbon == 'Review')
        Container(
          color: Colors.white,
          padding: const EdgeInsets.all(8),
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                if (spreadsheet) ...[
                  TextButton.icon(
                    key: const Key('sheet-new-comment'),
                    onPressed: () =>
                        guard(() => showSheetReviews(kind: 'comment')),
                    icon: const Icon(Icons.add_comment_outlined, size: 17),
                    label: const Text('New comment'),
                  ),
                  TextButton.icon(
                    key: const Key('sheet-new-note'),
                    onPressed: () =>
                        guard(() => showSheetReviews(kind: 'note')),
                    icon: const Icon(Icons.sticky_note_2_outlined, size: 17),
                    label: const Text('New note'),
                  ),
                  TextButton.icon(
                    key: const Key('sheet-reviews'),
                    onPressed: () => guard(showSheetReviews),
                    icon: const Icon(Icons.forum_outlined, size: 17),
                    label: const Text('Comments & notes'),
                  ),
                ],
                if (!spreadsheet) ...[
                  TextButton.icon(
                    key: const Key('document-tracking-toggle'),
                    onPressed: () => guard(toggleTracking),
                    icon: Icon(
                      recording?['enabled'] == true
                          ? Icons.fiber_manual_record
                          : Icons.edit_note,
                      size: 17,
                    ),
                    label: Text(
                      recording?['enabled'] == true
                          ? 'Recording on'
                          : 'Record changes',
                    ),
                  ),
                  IconButton(
                    key: const Key('document-tracking-settings'),
                    tooltip: 'Recording settings',
                    onPressed: () => guard(showTrackingSettings),
                    icon: const Icon(Icons.tune, size: 17),
                  ),
                  TextButton.icon(
                    key: const Key('document-revisions'),
                    onPressed: () => guard(showRevisions),
                    icon: const Icon(Icons.track_changes, size: 17),
                    label: const Text('Track Changes'),
                  ),
                  TextButton.icon(
                    key: const Key('document-review-view'),
                    onPressed: () => guard(showDocumentReview),
                    icon: const Icon(Icons.rate_review_outlined, size: 17),
                    label: const Text('Review in document'),
                  ),
                  TextButton.icon(
                    key: const Key('document-new-comment'),
                    onPressed: () => guard(() => showComments(add: true)),
                    icon: const Icon(Icons.add_comment_outlined, size: 17),
                    label: const Text('New comment'),
                  ),
                  TextButton.icon(
                    key: const Key('document-comments'),
                    onPressed: () => guard(showComments),
                    icon: const Icon(Icons.chat_bubble_outline, size: 17),
                    label: Text(
                      'Comments (${documentComments(model.workspace['document']).length})',
                    ),
                  ),
                ],
                if (!spreadsheet)
                  TextButton.icon(
                    onPressed: showWritingEditor,
                    icon: const Icon(Icons.spellcheck, size: 17),
                    label: const Text('Spelling & grammar'),
                  ),
                if (!spreadsheet)
                  TextButton.icon(
                    onPressed: agent.busy
                        ? null
                        : () => guard(() => checkGrammarWithAi(false)),
                    icon: const Icon(Icons.auto_awesome, size: 17),
                    label: const Text('AI grammar'),
                  ),
                TextButton.icon(
                  onPressed: () => guard(() => contextAction('ask')),
                  icon: const Icon(Icons.auto_awesome, size: 17),
                  label: const Text('Review selection'),
                ),
                TextButton.icon(
                  onPressed: () => guard(
                    () => fileCommand(WorkspaceFileCommand.compatibility),
                  ),
                  icon: const Icon(Icons.verified_outlined, size: 17),
                  label: const Text('Compatibility report'),
                ),
              ],
            ),
          ),
        ),
      if (anchor != null && !spreadsheet)
        Container(
          color: const Color(0xffedf3fc),
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
          child: Row(
            children: [
              const Icon(Icons.format_quote, size: 16, color: accent),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  anchor!.text.isEmpty
                      ? 'Insertion point selected'
                      : anchor!.text,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontSize: 11),
                ),
              ),
              TextButton(
                onPressed: () => guard(() => contextAction('ask')),
                child: const Text('Ask'),
              ),
              IconButton(
                tooltip: 'Clear selection context',
                onPressed: () => setState(() => anchor = null),
                icon: const Icon(Icons.close, size: 15),
              ),
            ],
          ),
        ),
      if (!spreadsheet && (recording != null || recordingNeedsAuthor))
        Container(
          color: const Color(0xffedf3fc),
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 4),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  recordingNeedsAuthor
                      ? 'This Word file requests Track Changes. Choose a review author before editing.'
                      : recording!['enabled'] == true
                      ? 'Recording changes as ${recording!['author']}'
                      : 'Recording is off. Earlier changes remain available for review.',
                  key: const Key('document-recording-status'),
                  style: const TextStyle(fontSize: 12),
                ),
              ),
              TextButton(
                onPressed: () => guard(showTrackingSettings),
                child: Text(
                  recordingNeedsAuthor ? 'Choose author' : 'Settings',
                ),
              ),
            ],
          ),
        ),
      if (!spreadsheet && _documentError != null)
        Container(
          key: const Key('document-edit-error'),
          color: const Color(0xfffff4e5),
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                'These edits have not been saved.',
                style: TextStyle(fontWeight: FontWeight.w600),
              ),
              Text(_documentError.toString()),
              const Text(
                'Your draft is still visible below. Copy it before discarding any unrecorded edits you want to keep.',
              ),
              Wrap(
                spacing: 8,
                children: [
                  TextButton(
                    onPressed: () async {
                      await Clipboard.setData(
                        ClipboardData(text: editor.document.toPlainText()),
                      );
                      notice('Draft text copied.');
                    },
                    child: const Text('Copy draft text'),
                  ),
                  TextButton(
                    key: const Key('discard-unrecorded-edits'),
                    onPressed: _discardingDocumentEdits
                        ? null
                        : () => guard(discardUnrecordedEdits),
                    child: const Text('Discard unrecorded edits'),
                  ),
                ],
              ),
            ],
          ),
        ),
      Expanded(
        child: IndexedStack(
          index: spreadsheet ? 1 : 0,
          children: [
            if (_reviewingDocument)
              DocumentReviewView(
                key: ValueKey('document-review-${model.documentSession}'),
                revision: model.revision,
                read: readDocumentReview,
                review: (request) async {
                  await flush();
                  await model.mutate({
                    'op': 'documentRevision',
                    ...request,
                  }, refreshDocument: true);
                },
                close: () {
                  editor.readOnly = recordingNeedsAuthor;
                  setState(() => _reviewingDocument = false);
                },
              )
            else
              DocumentView(
                key: documentKey,
                controller: editor,
                model: model,
                ribbon: ribbon,
                proofreading: proofreading,
                onBookmarks: () => guard(showBookmarks),
                onWritingFix: (issue, replacement) =>
                    guard(() => fixWriting(issue, replacement)),
                onContextAction: (action) => guard(() => contextAction(action)),
                onOpenSource: openSource,
                undo: () => guard(() => history(false)),
                redo: () => guard(() => history(true)),
              ),
            Column(
              children: [
                Expanded(
                  child: SheetView(
                    key: sheetKey,
                    onDefinedNames: () async {
                      final result = await model.core.call({
                        'op': 'definedNames',
                      });
                      return (result['names'] as List? ?? []).cast<Map>();
                    },
                    onGoToSheet: (target) async {
                      final name = target.sheetName!.toLowerCase();
                      final sheet = model.sheets.cast<Map?>().firstWhere(
                        (s) => (s!['name'] as String).toLowerCase() == name,
                        orElse: () => null,
                      );
                      if (sheet == null) return false;
                      await flush();
                      setState(() => sheetId = sheet['id']);
                      WidgetsBinding.instance.addPostFrameCallback(
                        (_) => sheetKey.currentState?.selectTarget(target),
                      );
                      return true;
                    },
                    sheet: currentSheet,
                    workspaceId: model.workspace['id'] as String?,
                    revision: model.revision,
                    calculationGeneration:
                        model.snapshot['evaluatedCells'] as int? ?? 0,
                    calculated:
                        (model.snapshot['calculated'] as Map?)?[sheetId] ?? {},
                    formattedCells:
                        (model.snapshot['formatted'] as Map?)?[sheetId] ?? {},
                    conditionalWarnings:
                        (model.snapshot['conditionalWarnings']
                            as Map?)?[sheetId] ??
                        [],
                    onConditionalRange: (request) =>
                        model.core.call({'op': 'conditionalRange', ...request}),
                    onConditionalCheck: (request) =>
                        model.core.call({'op': 'conditionalCheck', ...request}),
                    onConditionalFormat: (request) =>
                        model.mutate({'op': 'conditionalFormat', ...request}),
                    onNumberPreview: (request) => model.core.call({
                      'op': 'previewNumberFormat',
                      ...request,
                    }),
                    onChanges: (changes) =>
                        model.mutate({'op': 'cells', 'cells': changes}),
                    onEntryCheck: (request) => model.core.call({
                      'op': 'validationEntryCheck',
                      ...request,
                    }),
                    onEntry: (request) =>
                        model.mutate({'op': 'enterCell', ...request}),
                    onValidationChoices: (request) => model.core.call({
                      'op': 'validationChoices',
                      ...request,
                    }),
                    onValidation: (request) =>
                        model.mutate({'op': 'dataValidation', ...request}),
                    onReview: (address, kind) => guard(
                      () => showSheetReviews(address: address, kind: kind),
                    ),
                    onAudit: (address) =>
                        guard(() => showFormulaInspector(address)),
                    onWatch: (range) => guard(() => addWatch(range: range)),
                    onSelection: (s) => setState(() => selectedCell = s),
                    onStructure: changeSheetStructure,
                    onFill: (action) async {
                      await flush();
                      await model.mutate({
                        'op': 'fillCells',
                        'sheetId': sheetId,
                        'baseRevision': model.revision,
                        ...action,
                      }, refreshDocument: true);
                    },
                    onCopy: (action) async {
                      await flush();
                      return model.core.call({
                        'op': 'copyCells',
                        'baseRevision': model.revision,
                        ...action,
                      });
                    },
                    onPaste: (action) async {
                      await flush();
                      await model.mutate({
                        'op': 'pasteCells',
                        'baseRevision': model.revision,
                        ...action,
                      }, refreshDocument: true);
                    },
                    onGeometry: changeSheetGeometry,
                    onView: (action) async {
                      await flush();
                      await model.mutate({
                        'op': 'sheetView',
                        'sheetId': sheetId,
                        'baseRevision': model.revision,
                        ...action,
                      }, refreshDocument: true);
                    },
                    onTable: (action) async {
                      await flush();
                      await model.mutate({
                        'op': 'table',
                        'sheetId': sheetId,
                        'baseRevision': model.revision,
                        ...action,
                      }, refreshDocument: true);
                    },
                    onAutoFilter: (action) async {
                      await flush();
                      await model.mutate({
                        'op': 'autoFilter',
                        'sheetId': sheetId,
                        'baseRevision': model.revision,
                        ...action,
                      }, refreshDocument: true);
                    },
                    onFilterChoices: (action) async {
                      await flush();
                      return model.core.call({
                        'op': 'filterChoices',
                        'sheetId': sheetId,
                        ...action,
                      });
                    },
                    onSortRange: (action) async {
                      await flush();
                      await model.mutate({
                        'op': 'sortRange',
                        'sheetId': sheetId,
                        'baseRevision': model.revision,
                        ...action,
                      }, refreshDocument: true);
                    },
                    onFormat: (addresses, style, {baseRevision}) =>
                        model.mutate({
                          'op': 'formatCells',
                          'sheetId': sheetId,
                          'baseRevision': baseRevision ?? model.revision,
                          'addresses': addresses,
                          'style': style,
                        }),
                    onFormatText: (action) => model.mutate({
                      'op': 'formatCellText',
                      'sheetId': sheetId,
                      ...action,
                    }),
                  ),
                ),
                if (watchPanel)
                  WatchWindow(
                    workbook: model.workspace,
                    documentEpoch: model.documentEpoch,
                    calculated: model.snapshot['calculated'] as Map? ?? {},
                    formatted: model.snapshot['formatted'] as Map? ?? {},
                    onAdd: () => guard(addWatch),
                    onClose: () => setState(() => watchPanel = false),
                    onRemove: removeWatches,
                    onOpen: (id, a) => guard(() => openWatchedCell(id, a)),
                    onInspect: (id, a) =>
                        guard(() => openWatchedCell(id, a, inspect: true)),
                  ),
                Container(
                  height: 43,
                  color: const Color(0xfff5f6f7),
                  child: ListView(
                    scrollDirection: Axis.horizontal,
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 5,
                    ),
                    children: [
                      for (final s in model.sheets)
                        Padding(
                          padding: const EdgeInsets.only(right: 5),
                          child: GestureDetector(
                            onSecondaryTapDown: (details) => guard(
                              () => sheetMenu(s, details.globalPosition),
                            ),
                            child: TextButton.icon(
                              key: ValueKey('sheet-tab-${s['id']}'),
                              onPressed: () => guard(() async {
                                await sheetKey.currentState?.flush();
                                setState(() => sheetId = s['id']);
                              }),
                              icon: const Icon(
                                Icons.table_chart_outlined,
                                size: 13,
                              ),
                              label: Text(
                                s['name'],
                                style: const TextStyle(fontSize: 10),
                              ),
                              style: TextButton.styleFrom(
                                backgroundColor: s['id'] == sheetId
                                    ? const Color(0xffe3f0e8)
                                    : null,
                                foregroundColor: const Color(0xff107c41),
                              ),
                            ),
                          ),
                        ),
                      IconButton(
                        tooltip: 'Add sheet',
                        onPressed: () => guard(() async {
                          await flush();
                          await model.mutate({'op': 'addSheet'});
                          setState(() => sheetId = model.sheets.last['id']);
                        }),
                        icon: const Icon(Icons.add, size: 16),
                      ),
                      PopupMenuButton<String>(
                        key: const Key('sheet-actions'),
                        tooltip: 'Sheet options',
                        icon: const Icon(Icons.more_horiz, size: 18),
                        itemBuilder: (_) => sheetActions(currentSheet),
                        onSelected: (action) =>
                            guard(() => sheetAction(currentSheet, action)),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
      Container(
        height: 30,
        padding: const EdgeInsets.symmetric(horizontal: 20),
        decoration: const BoxDecoration(
          border: Border(top: BorderSide(color: line)),
        ),
        child: Row(
          children: [
            Text(
              '$_wordCount words',
              style: const TextStyle(fontSize: 9, color: muted),
            ),
            const SizedBox(width: 20),
            Text(
              'Revision ${model.revision}',
              style: const TextStyle(fontSize: 9, color: muted),
            ),
            const SizedBox(width: 12),
            Expanded(child: _statusSummary(spreadsheet)),
            const SizedBox(width: 20),
            const Text('100%', style: TextStyle(fontSize: 9, color: muted)),
          ],
        ),
      ),
    ],
  );

  /// Selection aggregates for a worksheet range (Average, Count and Sum
  /// by default; right-click to choose), otherwise the workspace summary.
  Widget _statusSummary(bool spreadsheet) {
    SelectionStats? stats;
    if (spreadsheet) {
      final sheet = currentSheet;
      final rows = VisibleRows(sheet), columns = VisibleColumns(sheet);
      stats = selectionStats(
        range: selectedCell,
        cells: sheet['cells'] as Map? ?? {},
        calculated: (model.snapshot['calculated'] as Map?)?[sheetId] ?? {},
        formatted: (model.snapshot['formatted'] as Map?)?[sheetId] ?? {},
        rowHidden: rows.isHidden,
        columnHidden: columns.isHidden,
      );
    }
    final text = stats == null
        ? (spreadsheet ? '1,048,576 rows · 16,384 columns' : 'Local workspace')
        : statusText(stats, _statusAggregates);
    return GestureDetector(
      key: const Key('status-summary'),
      onSecondaryTapDown: !spreadsheet
          ? null
          : (details) async {
              final chosen = await chooseAggregates(
                context,
                details.globalPosition,
                _statusAggregates,
              );
              if (chosen != null && mounted) {
                setState(() => _statusAggregates = chosen);
              }
            },
      child: Text(
        text,
        textAlign: TextAlign.right,
        overflow: TextOverflow.ellipsis,
        style: const TextStyle(fontSize: 9, color: muted),
      ),
    );
  }

  Widget _assistant() => Container(
    decoration: const BoxDecoration(
      color: Color(0xfffcfbf8),
      border: Border(left: BorderSide(color: line)),
    ),
    child: Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(22, 23, 22, 14),
          child: Row(
            children: [
              const Icon(Icons.auto_awesome_outlined, size: 19, color: accent),
              const SizedBox(width: 10),
              const Text(
                'Assistant',
                style: TextStyle(fontSize: 13, fontWeight: FontWeight.w500),
              ),
              const SizedBox(width: 9),
              _badge('BETA'),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 22),
          child: Row(
            children: [
              Icon(
                Icons.circle,
                size: 5,
                color: agent.signedIn ? const Color(0xff7b9570) : muted,
              ),
              const SizedBox(width: 6),
              Text(
                agent.signedIn ? 'Connected to Codex' : 'Codex disconnected',
                style: const TextStyle(fontSize: 10, color: muted),
              ),
              const Spacer(),
              TextButton(
                key: const Key('connect'),
                onPressed: () => guard(agent.login),
                child: Text(
                  agent.signedIn ? 'Reconnect' : 'Connect',
                  style: const TextStyle(fontSize: 10, color: accent),
                ),
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: ListView(
            controller: chatScroll,
            padding: const EdgeInsets.symmetric(horizontal: 23),
            children: [
              if (agent.messages.isEmpty &&
                  proposals.isEmpty &&
                  assistantUi.surfaces.isEmpty)
                _welcome(),
              if (attachedRange != null)
                Padding(
                  padding: const EdgeInsets.only(top: 10),
                  child: InputChip(
                    label: Text(
                      '${attachedRange!['sheetName']}!${attachedRange!['range']}',
                      style: const TextStyle(fontSize: 11),
                    ),
                    avatar: const Icon(Icons.grid_on, size: 15),
                    onDeleted: () => setState(() => attachedRange = null),
                    onPressed: () => openSource(
                      attachedRange!['sheetId'],
                      attachedRange!['range'],
                    ),
                  ),
                ),
              for (final id in assistantUi.surfaces)
                Container(
                  margin: const EdgeInsets.only(top: 12),
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    border: Border.all(color: line),
                    borderRadius: BorderRadius.circular(8),
                    color: Colors.white,
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Align(
                        alignment: Alignment.centerRight,
                        child: IconButton(
                          tooltip: 'Close assistant form',
                          onPressed: () => assistantUi.close(id),
                          icon: const Icon(Icons.close, size: 14),
                        ),
                      ),
                      assistantUi.surface(id),
                    ],
                  ),
                ),
              for (final m in agent.messages)
                Padding(
                  padding: const EdgeInsets.only(top: 22, bottom: 8),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        m.role == 'user' ? 'YOU' : 'FOLIO',
                        style: const TextStyle(
                          fontSize: 8,
                          letterSpacing: 1.3,
                          color: muted,
                        ),
                      ),
                      const SizedBox(height: 10),
                      SelectableText(
                        m.text,
                        style: const TextStyle(
                          fontSize: 12,
                          height: 1.7,
                          color: Color(0xff686151),
                        ),
                      ),
                    ],
                  ),
                ),
              for (final p in proposals) _proposal(p),
              if (agent.lastError != null)
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 14),
                  child: Text(
                    agent.lastError!,
                    style: const TextStyle(
                      fontSize: 11,
                      color: Color(0xffa76750),
                    ),
                  ),
                ),
            ],
          ),
        ),
        if (agent.busy)
          Padding(
            padding: const EdgeInsets.fromLTRB(23, 5, 23, 8),
            child: Row(
              children: [
                const SizedBox(
                  width: 10,
                  height: 10,
                  child: CircularProgressIndicator(strokeWidth: 1.5),
                ),
                const SizedBox(width: 9),
                Expanded(
                  child: Text(
                    agent.activity,
                    style: const TextStyle(fontSize: 10, color: muted),
                  ),
                ),
              ],
            ),
          ),
        Padding(
          padding: const EdgeInsets.fromLTRB(18, 8, 18, 14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                '•  ${spreadsheet
                    ? '${currentSheet['name']} · $selectedCell'
                    : anchor != null
                    ? 'Selected passage + workbook'
                    : 'Document + workbook'}',
                style: const TextStyle(fontSize: 9, color: muted),
              ),
              const SizedBox(height: 8),
              Container(
                padding: const EdgeInsets.fromLTRB(13, 7, 10, 9),
                decoration: BoxDecoration(
                  color: paper,
                  border: Border.all(color: const Color(0xffdcd5c6)),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Column(
                  children: [
                    Focus(
                      onKeyEvent: (_, e) {
                        if (e is KeyDownEvent &&
                            e.logicalKey == LogicalKeyboardKey.enter &&
                            !HardwareKeyboard.instance.isShiftPressed) {
                          guard(send);
                          return KeyEventResult.handled;
                        }
                        return KeyEventResult.ignored;
                      },
                      child: draftTextField(
                        key: const Key('prompt'),
                        controller: prompt,
                        minLines: 3,
                        maxLines: 6,
                        style: const TextStyle(fontSize: 12, height: 1.5),
                        decoration: const InputDecoration(
                          hintText: 'What would you like to work on?',
                          hintStyle: TextStyle(color: muted, fontSize: 12),
                          border: InputBorder.none,
                        ),
                      ),
                    ),
                    Row(
                      children: [
                        const Text(
                          'Codex',
                          style: TextStyle(fontSize: 9, color: muted),
                        ),
                        const Spacer(),
                        SizedBox(
                          width: 30,
                          height: 30,
                          child: IconButton(
                            key: const Key('send'),
                            onPressed: () =>
                                guard(agent.busy ? agent.stop : send),
                            style: IconButton.styleFrom(
                              backgroundColor: accent,
                              foregroundColor: Colors.white,
                              padding: EdgeInsets.zero,
                            ),
                            icon: Icon(
                              agent.busy
                                  ? Icons.stop_rounded
                                  : Icons.arrow_upward,
                              size: 18,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 10),
              const Text(
                'AI requests send workspace context to OpenAI.\nProposed edits stay yours to review.',
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 9, height: 1.5, color: muted),
              ),
            ],
          ),
        ),
      ],
    ),
  );
  Widget _welcome() => Padding(
    padding: const EdgeInsets.only(top: 42),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          width: 45,
          height: 45,
          decoration: BoxDecoration(
            color: const Color(0xfff0eadf),
            borderRadius: BorderRadius.circular(12),
          ),
          child: const Icon(
            Icons.auto_awesome_outlined,
            color: accent,
            size: 23,
          ),
        ),
        const SizedBox(height: 24),
        const Text(
          'Work with your\nwords and numbers.',
          style: TextStyle(
            fontFamily: 'Georgia',
            fontSize: 24,
            height: 1.2,
            letterSpacing: -.8,
            color: Color(0xff514c3e),
          ),
        ),
        const SizedBox(height: 17),
        const Text(
          'Write, explore, and make sense of your work—with an assistant that sees the whole workspace.',
          style: TextStyle(fontSize: 12, height: 1.8, color: muted),
        ),
        const SizedBox(height: 27),
        _suggestion(
          'Turn numbers into a story',
          Icons.description_outlined,
          'Read the workbook and propose a concise budget summary to append to the document.',
        ),
        _suggestion(
          'Find a clearer way to say it',
          Icons.auto_awesome_outlined,
          'Read the document and suggest three ways to improve its clarity. Do not edit yet.',
        ),
        _suggestion(
          'Make sense of this workbook',
          Icons.grid_on,
          'Read the workbook, check the formulas, and explain the budget. Do not edit yet.',
        ),
      ],
    ),
  );
  Widget _suggestion(String title, IconData icon, String text) => Padding(
    padding: const EdgeInsets.only(bottom: 9),
    child: OutlinedButton(
      onPressed: () => prompt.text = text,
      style: OutlinedButton.styleFrom(
        side: const BorderSide(color: line),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(7)),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 15),
      ),
      child: Row(
        children: [
          Icon(icon, size: 14, color: muted),
          const SizedBox(width: 9),
          Expanded(
            child: Text(
              title,
              style: const TextStyle(fontSize: 11, color: Color(0xff7f745f)),
            ),
          ),
          const Icon(Icons.arrow_forward, size: 12, color: muted),
        ],
      ),
    ),
  );
  Widget _proposal(Map<String, dynamic> p) {
    final status = p['_status'];
    return Container(
      key: const Key('proposal'),
      margin: const EdgeInsets.symmetric(vertical: 15),
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: const Color(0xfff5f7ef),
        border: Border.all(color: const Color(0xffd7dccb)),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Material(
        color: Colors.transparent,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              '✧  PROPOSED CHANGES',
              style: TextStyle(
                fontSize: 8,
                letterSpacing: 1,
                color: Color(0xff8b9576),
              ),
            ),
            const SizedBox(height: 9),
            Text(
              p['summary'],
              style: const TextStyle(fontSize: 12, height: 1.5),
            ),
            const SizedBox(height: 10),
            ExpansionTile(
              tilePadding: EdgeInsets.zero,
              childrenPadding: EdgeInsets.zero,
              title: const Text(
                'Review changes',
                style: TextStyle(fontSize: 11),
              ),
              children: [
                for (final edit in p['proofreadEdits'] ?? [])
                  Padding(
                    padding: const EdgeInsets.only(bottom: 12),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Text(
                          edit['expectedText'],
                          style: const TextStyle(
                            color: Color(0xffa4262c),
                            decoration: TextDecoration.lineThrough,
                          ),
                        ),
                        Text(
                          edit['replacement'].toString().isEmpty
                              ? '(Remove)'
                              : edit['replacement'],
                          style: const TextStyle(color: Color(0xff107c41)),
                        ),
                      ],
                    ),
                  ),
                if (p['replaceSelection'] != null) ...[
                  const Text(
                    'Selected passage · before',
                    style: TextStyle(fontSize: 11, color: Colors.red),
                  ),
                  SelectableText(p['_selectionBefore'] ?? ''),
                  const SizedBox(height: 8),
                  const Text(
                    'Proposed replacement',
                    style: TextStyle(fontSize: 11, color: Color(0xff107c41)),
                  ),
                  SelectableText(
                    p['replaceSelection']['text'],
                    style: const TextStyle(fontSize: 12, height: 1.5),
                  ),
                ],
                if (p['linkedBlock'] != null)
                  WorkbookBlock(
                    model: model,
                    attributes: p['linkedBlock'],
                    onOpen: openSource,
                  ),
                if (p['replaceDocument'] != null) ...[
                  const Text(
                    'Replace all document content and formatting:',
                    style: TextStyle(fontSize: 11, color: Colors.brown),
                  ),
                  SelectableText(
                    (p['replaceDocument'] as List).join('\n\n'),
                    style: const TextStyle(fontSize: 12),
                  ),
                ],
                if (p['appendParagraphs'] != null) ...[
                  const Text(
                    'Append paragraphs:',
                    style: TextStyle(fontSize: 11),
                  ),
                  SelectableText(
                    (p['appendParagraphs'] as List).join('\n\n'),
                    style: const TextStyle(fontSize: 12, height: 1.6),
                  ),
                ],
                for (final c in p['cells'] ?? [])
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 5),
                    child: Align(
                      alignment: Alignment.centerLeft,
                      child: Text(
                        '${c['sheetId']}!${c['address']}\n${_beforeCell(p, c)} → ${c['value']}',
                        style: const TextStyle(fontSize: 11, height: 1.6),
                      ),
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 8),
            if (status == 'pending' && p['baseRevision'] != model.revision)
              const Text(
                'The workspace has changed. Ask for an updated proposal.',
                style: TextStyle(fontSize: 11, color: Colors.orange),
              ),
            if (status == 'pending')
              Wrap(
                children: [
                  FilledButton(
                    key: const Key('apply-proposal'),
                    onPressed: p['baseRevision'] == model.revision
                        ? () => guard(() => apply(p))
                        : null,
                    style: FilledButton.styleFrom(
                      backgroundColor: const Color(0xff83936c),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(5),
                      ),
                    ),
                    child: const Text(
                      'Apply changes',
                      style: TextStyle(fontSize: 11),
                    ),
                  ),
                  TextButton(
                    onPressed: () => setState(() => p['_status'] = 'dismissed'),
                    child: const Text(
                      'Dismiss',
                      style: TextStyle(fontSize: 10),
                    ),
                  ),
                ],
              )
            else
              Text(
                status == 'applied'
                    ? 'Applied · included in workspace history'
                    : 'Dismissed',
                style: const TextStyle(fontSize: 10, color: Color(0xff7e8b6a)),
              ),
          ],
        ),
      ),
    );
  }

  String _beforeCell(Map p, Map c) {
    final s = (p['_before']['sheets'] as List).where(
      (s) => s['id'] == c['sheetId'],
    );
    return s.isEmpty
        ? ''
        : s.first['cells'][c['address']]?.toString() ?? '(empty)';
  }

  @override
  void dispose() {
    FocusManager.instance.removeListener(_syncHistoryMenu);
    focusedDraftHistory.removeListener(_syncHistoryMenu);
    historyMenuChannel.setMethodCallHandler(null);
    _workspaceFocus.dispose();
    lifecycleChannel.setMethodCallHandler(null);
    fileMenuChannel.setMethodCallHandler(null);
    _lifecycle?.dispose();
    _calendarRefresh.dispose();
    model.removeListener(_modelChanged);
    agent.removeListener(_agentChanged);
    assistantUi.dispose();
    agent.dispose();
    _inputSubscription?.cancel();
    editor.removeListener(_documentChanged);
    proofreading.removeListener(_proofreadingChanged);
    proofreading.dispose();
    editor.dispose();
    prompt.dispose();
    chatScroll.dispose();
    super.dispose();
  }
}

class RecalculateIntent extends Intent {
  const RecalculateIntent();
}
