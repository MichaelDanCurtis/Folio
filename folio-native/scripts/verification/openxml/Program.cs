using DocumentFormat.OpenXml;
using DocumentFormat.OpenXml.Packaging;
using DocumentFormat.OpenXml.Validation;
using System.Text.Json;
var results = new List<object>();
var invalid = false;
foreach (var file in args) {
    using OpenXmlPackage doc = Path.GetExtension(file).ToLowerInvariant() is ".xlsx" or ".xltx"
        ? SpreadsheetDocument.Open(file, false)
        : WordprocessingDocument.Open(file, false);
    var errors = new OpenXmlValidator(FileFormatVersions.Microsoft365).Validate(doc).Select(e => new {e.Description, Path=e.Path?.XPath, Part=e.Part?.Uri.ToString()}).ToArray();
    invalid |= errors.Length > 0;
    results.Add(new {file, errors});
}
Console.WriteLine(JsonSerializer.Serialize(results,new JsonSerializerOptions{WriteIndented=true}));

Environment.ExitCode = invalid ? 1 : 0;
