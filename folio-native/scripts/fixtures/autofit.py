"""Independent wrapped/rotated worksheet for row-fitting UI qualification."""
from pathlib import Path
from openpyxl import Workbook
from openpyxl.styles import Font, Alignment, PatternFill
from openpyxl.cell.rich_text import CellRichText, TextBlock
from openpyxl.cell.text import InlineFont
from openpyxl.comments import Comment

w = Workbook(); s = w.active; s.title = 'Delivery plan'
rows = [
    ['Workstream', 'Review notes', 'Owner'],
    ['Research', 'Interview customers, review the findings, and share a concise summary with the product team.', 'Maya'],
    ['Design', 'First concept\nAccessibility review\nFinal handoff', 'Alex'],
    ['Prototype', CellRichText('Build ', TextBlock(InlineFont(rFont='Georgia', sz=20, b=True, color='167046'), 'together'), ' and test the details.'), 'Sam'],
    ['Rollout', 'Prepare release notes and a recovery plan.', 'Jordan'],
    ['Hidden source', 'A hidden row must retain its height.', 'Private'],
]
for row in rows: s.append(row)
for row in s:
    for c in row:
        c.font = Font(name='Georgia', size=11, color='243347')
        c.alignment = Alignment(wrap_text=True, vertical='top')
for c in s[1]:
    c.font = Font(name='Georgia', size=12, bold=True, color='FFFFFF')
    c.fill = PatternFill('solid', fgColor='176B48')
s['A5'].alignment = Alignment(textRotation=30, vertical='top')
s['B2'].comment = Comment('Keep this note through AutoFit.', 'Folio test')
s['Z2'] = 'Offscreen\nSecond line\nThird line\nFourth line'
s['Z2'].font = Font(name='Georgia', size=11)
s.row_dimensions[6].hidden = True; s.row_dimensions[6].height = 75
for r in range(1,6): s.row_dimensions[r].height = 18
for c, width in [('A',20),('B',33),('C',20),('Z',18)]: s.column_dimensions[c].width=width
s.freeze_panes = 'A2'
w.save(Path(__file__).resolve().parents[2]/'test/fixtures/autofit.xlsx')
print('Created autofit.xlsx')
