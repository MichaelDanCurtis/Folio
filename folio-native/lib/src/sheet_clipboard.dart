import 'package:flutter/services.dart';

/// Native cell data is an opaque, process-local token. Only the selected text
/// goes on the public clipboard; an attached source workbook never does.
class SheetClipboard {
  static const channel = MethodChannel('folio/clipboard');
  static Future<void> write(Map<String, dynamic> copied) async {
    try {
      await channel.invokeMethod<void>('write', {
        'text': copied['text'],
        'token': copied['token'],
      });
    } on MissingPluginException {
      await Clipboard.setData(ClipboardData(text: copied['text'] as String));
    }
  }

  static Future<Map<String, dynamic>?> read() async {
    try {
      final value = await channel.invokeMapMethod<String, dynamic>('read');
      return value == null ? null : Map<String, dynamic>.from(value);
    } on MissingPluginException {
      final data = await Clipboard.getData('text/plain');
      return data?.text == null ? null : {'text': data!.text};
    }
  }
}
