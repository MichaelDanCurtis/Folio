"""Independent worksheet range source without table metadata."""
from pathlib import Path
from openpyxl import Workbook
from openpyxl.styles import Font
from openpyxl.comments import Comment
from openpyxl.worksheet.hyperlink import Hyperlink
from openpyxl.cell.rich_text import CellRichText, TextBlock
from openpyxl.cell.text import InlineFont

root=Path(__file__).resolve().parents[2]
b=Workbook();s=b.active;s.title='Records'
s.append(['Label','Qty','Rate','Amount','Outside'])
for r,(label,qty,rate) in enumerate([('Pear',3,5),('Apple',2,4),('Hidden',99,2),('Pear',1,8),(None,7,1),('Pear',1,3)],2):
    s.append([label,qty,rate,f'=B{r}*C{r}',f'fixed-{r}'])
s['A2']=CellRichText(TextBlock(InlineFont(b=True),'Pe'),'ar')
s['A2'].comment=Comment('First pear record','Folio test')
s['A2'].hyperlink=Hyperlink(ref='A2',location="'Summary'!A1")
s['B2'].font=Font(bold=True,color='FF003399')
s['D2'].number_format='$#,##0.00'
s.row_dimensions[4].hidden=True
s.freeze_panes='B2'
q=b.create_sheet('Summary');q['A1']='=SUM(Records!D2:D7)';q['B1']='=Records!A2'
b.save(root/'test/fixtures/range-sort.xlsx')
print('Wrote range-sort.xlsx')
