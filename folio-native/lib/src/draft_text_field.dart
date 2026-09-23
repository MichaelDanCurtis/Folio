import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'history_commands.dart';

/// Standard Flutter fields with immediate, field-local draft history.
/// Keep the key on the actual field for accessibility and existing callers.
Widget draftTextField({
  Key? key,
  TextEditingController? controller,
  String? initialValue,
  FocusNode? focusNode,
  bool formField = false,
  bool autofocus = false,
  bool? enabled,
  bool readOnly = false,
  int? minLines,
  int? maxLines = 1,
  int? maxLength,
  TextStyle? style,
  TextInputType? keyboardType,
  TextInputAction? textInputAction,
  List<TextInputFormatter>? inputFormatters,
  InputDecoration? decoration = const InputDecoration(),
  ValueChanged<String>? onChanged,
  ValueChanged<String>? onSubmitted,
  ValueChanged<String>? onFieldSubmitted,
}) {
  assert(controller == null || initialValue == null);
  return _DraftField(
    key: key == null ? null : ValueKey((#draftField, key)),
    controller: controller,
    initialValue: initialValue ?? '',
    focusNode: focusNode,
    readOnly: readOnly || enabled == false,
    builder: (value, focus) => formField
        ? TextFormField(
            key: key,
            controller: value,
            focusNode: focus,
            autofocus: autofocus,
            enabled: enabled,
            readOnly: readOnly,
            minLines: minLines,
            maxLines: maxLines,
            maxLength: maxLength,
            style: style,
            keyboardType: keyboardType,
            textInputAction: textInputAction,
            inputFormatters: inputFormatters,
            decoration: decoration,
            onChanged: onChanged,
            onFieldSubmitted: onFieldSubmitted,
          )
        : TextField(
            key: key,
            controller: value,
            focusNode: focus,
            autofocus: autofocus,
            enabled: enabled,
            readOnly: readOnly,
            minLines: minLines,
            maxLines: maxLines,
            maxLength: maxLength,
            style: style,
            keyboardType: keyboardType,
            textInputAction: textInputAction,
            inputFormatters: inputFormatters,
            decoration: decoration,
            onChanged: onChanged,
            onSubmitted: onSubmitted,
          ),
  );
}

class _DraftField extends StatefulWidget {
  final TextEditingController? controller;
  final FocusNode? focusNode;
  final String initialValue;
  final bool readOnly;
  final Widget Function(TextEditingController, FocusNode) builder;
  const _DraftField({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.initialValue,
    required this.readOnly,
    required this.builder,
  });
  @override
  State<_DraftField> createState() => _DraftFieldState();
}

class _DraftFieldState extends State<_DraftField> {
  late TextEditingController _controller;
  late FocusNode _focus;
  late _DraftHistory _history;
  bool _applying = false;

  @override
  void initState() {
    super.initState();
    _controller =
        widget.controller ?? TextEditingController(text: widget.initialValue);
    _focus = widget.focusNode ?? FocusNode();
    _history = _DraftHistory(_controller.value);
    _controller.addListener(_changed);
    _focus.addListener(_focusChanged);
  }

  void _changed() {
    if (_applying) return;
    _history.record(_controller.value, editing: _focus.hasFocus);
    _publishHistory();
  }

  void _focusChanged() {
    if (!_focus.hasFocus) _history.commit();
    _publishHistory();
  }

  void _publishHistory() {
    final previous = focusedDraftHistory.value;
    if (!_focus.hasFocus) {
      if (identical(previous?.owner, this)) focusedDraftHistory.value = null;
      return;
    }
    final editable =
        !widget.readOnly && _controller.value.composing.isCollapsed;
    final undo = editable && _history.canUndo;
    final redo = editable && _history.canRedo;
    if (identical(previous?.owner, this) &&
        previous?.canUndo == undo &&
        previous?.canRedo == redo) {
      return;
    }
    focusedDraftHistory.value = DraftHistoryTarget(
      owner: this,
      canUndo: undo,
      canRedo: redo,
      undo: () => _restore(false),
      redo: () => _restore(true),
    );
  }

  void _restore(bool redo) {
    if (widget.readOnly ||
        !_focus.hasFocus ||
        !_controller.value.composing.isCollapsed) {
      return;
    }
    final editable = _focus.context
        ?.findAncestorStateOfType<EditableTextState>();
    if (editable == null) return;
    final value = redo ? _history.redo() : _history.undo();
    if (value == null) return;
    _applying = true;
    try {
      // Preserve Flutter's selection, input connection, formatters and onChanged.
      editable.userUpdateTextEditingValue(
        value,
        SelectionChangedCause.keyboard,
      );
      _history.acceptRestored(_controller.value);
    } finally {
      _applying = false;
      _publishHistory();
    }
  }

  @override
  void didUpdateWidget(_DraftField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.controller != oldWidget.controller) {
      final previous = _controller;
      previous.removeListener(_changed);
      _controller =
          widget.controller ?? TextEditingController.fromValue(previous.value);
      if (oldWidget.controller == null) previous.dispose();
      _history.reset(_controller.value);
      _controller.addListener(_changed);
    }
    if (widget.focusNode != oldWidget.focusNode) {
      _focus.removeListener(_focusChanged);
      if (oldWidget.focusNode == null) _focus.dispose();
      _focus = widget.focusNode ?? FocusNode();
      _focus.addListener(_focusChanged);
      _history.commit();
    }
    _publishHistory();
  }

  @override
  Widget build(BuildContext context) => Actions(
    // EditableText's overridable actions delegate here. Empty draft history is
    // still handled locally, so it cannot fall through to a workspace edit.
    actions: {
      UndoTextIntent: CallbackAction<UndoTextIntent>(
        onInvoke: (_) {
          _restore(false);
          return null;
        },
      ),
      RedoTextIntent: CallbackAction<RedoTextIntent>(
        onInvoke: (_) {
          _restore(true);
          return null;
        },
      ),
    },
    child: widget.builder(_controller, _focus),
  );

  @override
  void dispose() {
    if (identical(focusedDraftHistory.value?.owner, this)) {
      focusedDraftHistory.value = null;
    }
    _history.dispose();
    _controller.removeListener(_changed);
    _focus.removeListener(_focusChanged);
    if (widget.controller == null) _controller.dispose();
    if (widget.focusNode == null) _focus.dispose();
    super.dispose();
  }
}

class _DraftHistory {
  final List<TextEditingValue> _entries = [];
  int _index = 0;
  late TextEditingValue _observed;
  TextEditingValue? _pending;
  Timer? _timer;

  _DraftHistory(TextEditingValue initial) {
    reset(initial);
  }

  bool get canUndo =>
      _index > 0 ||
      (_pending != null && _pending!.text != _entries[_index].text);
  bool get canRedo => _pending == null && _index + 1 < _entries.length;

  TextEditingValue _snapshot(TextEditingValue value) => value.copyWith(
    selection: value.selection.isValid
        ? TextSelection(
            baseOffset: value.selection.baseOffset.clamp(0, value.text.length),
            extentOffset: value.selection.extentOffset.clamp(
              0,
              value.text.length,
            ),
            affinity: value.selection.affinity,
            isDirectional: value.selection.isDirectional,
          )
        : TextSelection.collapsed(offset: value.text.length),
    composing: TextRange.empty,
  );

  void reset(TextEditingValue value) {
    _timer?.cancel();
    _timer = null;
    _pending = null;
    _entries
      ..clear()
      ..add(_snapshot(value));
    _index = 0;
    _observed = value;
  }

  void record(TextEditingValue value, {required bool editing}) {
    // A value supplied outside editing starts a new draft. Selection-only
    // notifications on blur must not erase the user's existing Undo history.
    if (!editing &&
        value.text != _observed.text &&
        _observed.composing.isCollapsed) {
      reset(value);
      return;
    }
    final previous = _observed;
    _observed = value;
    if (!value.composing.isCollapsed) {
      // Keep an IME composition together and leave live composing text intact.
      if (previous.composing.isCollapsed) commit();
      return;
    }
    if (value.text == previous.text && previous.composing.isCollapsed) {
      if (value.selection != previous.selection) commit();
      if (_entries[_index].text == value.text) {
        _entries[_index] = _snapshot(value);
      }
      return;
    }
    if (!previous.composing.isCollapsed &&
        value.text == _entries[_index].text) {
      // Cancelling an uncommitted composition does not create a new branch.
      _entries[_index] = _snapshot(value);
      return;
    }
    // A new edit branches history immediately, before the coalescing timer fires.
    _entries.removeRange(_index + 1, _entries.length);
    _pending = _snapshot(value);
    if (!previous.composing.isCollapsed) {
      commit();
    } else {
      _timer ??= Timer(const Duration(milliseconds: 500), commit);
    }
  }

  void commit() {
    _timer?.cancel();
    _timer = null;
    final value = _pending;
    _pending = null;
    if (value == null) return;
    if (_entries[_index].text == value.text) {
      _entries[_index] = value;
    } else {
      _entries.add(value);
      _index = _entries.length - 1;
    }
  }

  TextEditingValue? undo() {
    // Commit the latest text before moving back so an immediate Redo retains it.
    commit();
    if (_index == 0) return null;
    return _entries[--_index];
  }

  TextEditingValue? redo() {
    commit();
    if (_index + 1 == _entries.length) return null;
    return _entries[++_index];
  }

  void acceptRestored(TextEditingValue value) {
    _observed = value;
    _entries[_index] = _snapshot(value);
  }

  void dispose() => _timer?.cancel();
}
