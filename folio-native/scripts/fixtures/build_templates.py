"""Build Folio-authored starter templates; run with bundled python-docx/openpyxl."""
from pathlib import Path
from io import BytesIO
from zipfile import ZipFile, ZIP_DEFLATED
from datetime import datetime
from docx import Document
from docx.shared import Inches, Pt, RGBColor
from openpyxl import Workbook
from openpyxl.styles import Font, PatternFill, Alignment
from openpyxl.worksheet.datavalidation import DataValidation

root = Path(__file__).resolve().parents[2] / 'assets/templates'
root.mkdir(parents=True, exist_ok=True)

def word(name, title, subtitle, sections):
    doc = Document()
    section = doc.sections[0]
    section.page_width, section.page_height = Inches(8.5), Inches(11)
    section.top_margin = section.bottom_margin = Inches(0.8)
    doc.styles['Normal'].font.name = 'Aptos'
    doc.styles['Normal'].font.size = Pt(11)
    doc.styles['Normal'].paragraph_format.space_after = Pt(8)
    for style in ['Title', 'Heading 1', 'Heading 2']:
        doc.styles[style].font.color.rgb = RGBColor.from_string('185ABD')
    doc.add_heading(title, 0)
    doc.add_paragraph(subtitle, 'Subtitle')
    for heading, body in sections:
        if heading:
            doc.add_heading(heading, 1)
        if isinstance(body, list):
            table = doc.add_table(rows=1, cols=len(body[0]))
            table.style = 'Light Shading Accent 1'
            for cell, value in zip(table.rows[0].cells, body[0]):
                cell.text = value
            for row in body[1:]:
                for cell, value in zip(table.add_row().cells, row):
                    cell.text = value
        else:
            doc.add_paragraph(body)
    doc.core_properties.author = 'Folio'
    doc.core_properties.title = title
    doc.core_properties.created = doc.core_properties.modified = datetime(2026, 1, 1)
    data = BytesIO(); doc.save(data)
    with ZipFile(data) as original, ZipFile(root / name, 'w', ZIP_DEFLATED) as result:
        for item in original.infolist():
            content = original.read(item.filename)
            if item.filename == '[Content_Types].xml':
                content = content.replace(b'wordprocessingml.document.main+xml', b'wordprocessingml.template.main+xml')
            result.writestr(item, content)

word('letter.dotx', 'Business letter', '[Your name or organization]', [
    ('', '[Date]\n[Recipient name]\n[Organization]\n[Address]'),
    ('', 'Dear [Recipient],'),
    ('', '[Write the purpose of your letter here.]'),
    ('', '[Add the details and the action you would like the reader to take.]'),
    ('', 'Sincerely,\n[Your name]\n[Title and contact details]'),
])
word('report.dotx', 'Project report', '[Project name] · [Reporting period]', [
    ('Summary', '[Describe the outcome, current status, and next decision.]'),
    ('Progress', '[Record completed work and the evidence that supports it.]'),
    ('Results', [['Measure', 'Target', 'Actual'], ['[Measure]', '[Target]', '[Result]']]),
    ('Risks and next steps', [['Action', 'Owner', 'Due date'], ['[Next action]', '[Owner]', '[Date]']]),
])
word('minutes.dotx', 'Meeting notes', '[Meeting name] · [Date and time]', [
    ('Participants', '[Names and roles]'), ('Agenda', '[Topics to discuss]'),
    ('Discussion and decisions', '[Record the discussion and agreed decisions.]'),
    ('Action items', [['Action', 'Owner', 'Due date'], ['[Next action]', '[Owner]', '[Date]']]),
    ('Next meeting', '[Date, time, and proposed agenda]'),
])

def workbook(name, title, headers, rows, widths):
    book = Workbook(); book.template = True
    sheet = book.active; sheet.title = title
    sheet.append(headers)
    for row in rows: sheet.append(row)
    for cell in sheet[1]:
        cell.font = Font(name='Aptos', bold=True, color='FFFFFF')
        cell.fill = PatternFill('solid', fgColor='107C41')
        cell.alignment = Alignment(vertical='center')
    sheet.row_dimensions[1].height = 26
    for col, width in widths.items(): sheet.column_dimensions[col].width = width
    sheet.freeze_panes = 'A2'
    sheet.auto_filter.ref = f'A1:{chr(64 + len(headers))}{len(rows) + 1}'
    book.properties.creator = 'Folio'; book.properties.title = title
    book.properties.created = book.properties.modified = datetime(2026, 1, 1)
    return book, sheet

book, sheet = workbook('budget.xltx', 'Budget', ['Category', 'Planned', 'Actual', 'Remaining'], [
    [category, 0, 0, f'=B{i}-C{i}'] for i, category in enumerate(['Personnel', 'Equipment', 'Travel', 'Supplies', 'Services', 'Other'], 2)
], {'A': 26, 'B': 18, 'C': 18, 'D': 18})
sheet.append(['Total', '=SUM(B2:B7)', '=SUM(C2:C7)', '=SUM(D2:D7)'])
for row in sheet.iter_rows(min_row=2, min_col=2, max_col=4):
    for cell in row: cell.number_format = '$#,##0.00;[Red]($#,##0.00)'
for cell in sheet[8]: cell.font = Font(bold=True)
book.save(root / 'budget.xltx')

book, sheet = workbook('invoice.xltx', 'Invoice', ['Description', 'Quantity', 'Unit price', 'Amount'], [
    [f'[Item {i-1}]', 0, 0, f'=B{i}*C{i}'] for i in range(2, 8)
], {'A': 36, 'B': 16, 'C': 18, 'D': 18})
sheet['A9'] = 'Subtotal'; sheet['D9'] = '=SUM(D2:D7)'
sheet['A10'] = 'Tax rate'; sheet['C10'] = 0; sheet['C10'].number_format = '0.00%'
sheet['D10'] = '=D9*C10'; sheet['A11'] = 'Total'; sheet['D11'] = '=D9+D10'
sheet['A14'] = 'Invoice number'; sheet['B14'] = '[Number]'
sheet['A15'] = 'Bill to'; sheet['B15'] = '[Customer]'
sheet['A16'] = 'Payment terms'; sheet['B16'] = '[Terms]'
for row in sheet.iter_rows(min_row=2, max_row=11, min_col=3, max_col=4):
    for cell in row:
        if cell.coordinate != 'C10': cell.number_format = '$#,##0.00'
for cell in sheet[11]: cell.font = Font(bold=True)
book.save(root / 'invoice.xltx')

book, sheet = workbook('tracker.xltx', 'Project tracker', ['Task', 'Owner', 'Status', 'Due date', 'Notes'], [
    [f'[Task {i}]', '[Owner]', 'Not started', None, ''] for i in range(1, 9)
], {'A': 30, 'B': 22, 'C': 20, 'D': 17, 'E': 38})
validation = DataValidation(type='list', formula1='"Not started,In progress,Blocked,Complete"', allow_blank=True)
validation.errorTitle = 'Choose a status'; validation.error = 'Choose one of the listed statuses.'
validation.showErrorMessage = True; validation.errorStyle = 'stop'
sheet.add_data_validation(validation); validation.add('C2:C1000')
for row in range(2, 10): sheet[f'D{row}'].number_format = 'yyyy-mm-dd'
book.save(root / 'tracker.xltx')
print('Created six Folio-authored Word and Excel starter templates.')
