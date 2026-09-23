"""Independent XLSX inputs for display formatting and retained date systems."""
from pathlib import Path
from openpyxl import Workbook
from openpyxl.styles import Font, Alignment
from openpyxl.utils.datetime import CALENDAR_MAC_1904

root = Path(__file__).resolve().parents[2]
formats = [
    (6, '0.0000'), (1234.5, 'General'), (.12345, '0.00%'),
    (-1234.5, '#,##0.00;[Red](#,##0.00)'), (0, '0;[Red](0);"—"'),
    (123, ';;;'), (60, 'yyyy-mm-dd'), (0.9999999, 'h:mm:ss'),
    (2.5, '[h]:mm:ss'), (2.125, '# ??/??'), (12345, '0.00E+00'),
    (123456789, '#,##0,,"M"'), (1234.5, '[$€-407]#,##0.00'),
    ('hello', '0;0;0;"Note: "@'), (True, '0;0;0;"Note: "@'),
    (-1.25, '[>=100]0.0" large";[Red][<0](0.00);0.000'),
    (12.5, '[>=100]0.0" large";[Red][<0](0.00);0.000'),
    (1234.5, '_($* #,##0.00_);_($* (#,##0.00);_($* "-"??_);_(@_)'),
    (15, '[Color3]0.00'), ('=B2+1', '0.0000'),
]
for date1904 in [False, True]:
    wb = Workbook()
    if date1904: wb.epoch = CALENDAR_MAC_1904
    ws = wb.active
    ws.title = 'Formats'
    ws.append(['Example', 'Value', 'Format code'])
    for i, (value, code) in enumerate(formats, 2):
        ws.cell(i, 1, f'Case {i-1}')
        cell = ws.cell(i, 2, value)
        cell.number_format = code
        cell.font = Font(name='Georgia', size=12)
        ws.cell(i, 3, code)
    ws.column_dimensions['A'].width = 17
    ws.column_dimensions['B'].width = 30
    ws.column_dimensions['C'].width = 40
    ws.column_dimensions['E'].number_format = '0.0000'
    ws['E2'] = 4
    ws.row_dimensions[25].number_format = '0.00%'
    ws['E25'] = .25
    ws['B2'].alignment = Alignment(wrap_text=True)
    for cell in ['B22','B23','B24']:
        ws[cell] = {'B22': 0, 'B23': 1, 'B24': 1462}[cell]
        ws[cell].number_format = 'yyyy-mm-dd'
    wb.save(root / 'test' / 'fixtures' / ('number-formats-1904.xlsx' if date1904 else 'number-formats.xlsx'))
