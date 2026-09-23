"""Independent worksheet-view fixture: scroll origin differs from frozen boundary."""
from pathlib import Path
from openpyxl import Workbook
from openpyxl.worksheet.views import Pane, SheetView, Selection
from openpyxl.workbook.views import BookView
from openpyxl.comments import Comment
from openpyxl.styles import Font

root = Path(__file__).resolve().parents[2]
w = Workbook()
s = w.active
s.title = 'Frozen'
for r in range(1, 51):
    s.append([f'Row {r}', r, r * 10, f'=B{r}+C{r}', 'kept'])
s['A1'].font = Font(bold=True, color='185ABD')
s['A1'].comment = Comment('Preserve this note', 'Fixture')
s.freeze_panes = 'C4'
s.sheet_view.pane.topLeftCell = 'G20'
s.sheet_view.pane.state = 'frozenSplit'
s.sheet_view.zoomScale = 90
s.sheet_view.selection[-1] = Selection(pane='bottomRight', activeCell='H22', sqref='H22')
s.row_dimensions[1].height = 24
s.row_dimensions[2].hidden = True
s.row_dimensions[3].height = 40
s.column_dimensions['A'].width = 22
s.column_dimensions['B'].hidden = True
w.views.append(BookView())
s.views.sheetView.append(SheetView(workbookViewId=1, zoomScale=70,
    pane=Pane(xSplit=1, topLeftCell='B1', state='frozen', activePane='topRight')))
split = w.create_sheet('Split')
split['A1'] = 'Retain ordinary split panes'
split.sheet_view.pane = Pane(xSplit=2430, ySplit=1200, topLeftCell='D12', state='split', activePane='bottomRight')
w.create_sheet('Plain')['A1'] = 'No panes'
w.save(root/'test/fixtures/views.xlsx')
