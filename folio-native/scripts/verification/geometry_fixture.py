"""Independent fixture for sparse row sizes and grouped column metadata."""
from pathlib import Path
from openpyxl import load_workbook
from openpyxl.worksheet.dimensions import ColumnDimension
from openpyxl.styles import Font
root = Path(__file__).resolve().parents[2]
w = load_workbook(root/'test/fixtures/range-sort.xlsx',rich_text=True)
s = w['Records']
s.row_dimensions[2].height = 30
s.row_dimensions[5].height = 18
s.row_dimensions[25].height = 44
s.column_dimensions['A'].width = 24
dim = ColumnDimension(s,index='B',min=2,max=4,width=18,hidden=True,outlineLevel=1,collapsed=True)
dim.font = Font(bold=True,color='773355')
s.column_dimensions['B'] = dim
w.save(root/'test/fixtures/geometry.xlsx')
