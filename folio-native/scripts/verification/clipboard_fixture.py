from pathlib import Path
from openpyxl import Workbook
from openpyxl.styles import Font, Alignment, PatternFill, Border, Side
w=Workbook();s=w.active;s.title='Copy source'
s.append(['Units','Calculated','Multiline text','Text number'])
s.append([2,'=A2*3','One\nTwo\t"quoted"','001'])
s['B2'].font=Font(name='Georgia',size=16,bold=True,color='AA0000')
s['B2'].number_format='0.0000';s['B2'].fill=PatternFill('solid',fgColor='FFF2CC')
s['B2'].border=Border(bottom=Side(style='double',color='225533'))
s['C2'].alignment=Alignment(wrap_text=True,vertical='center')
for column,width in [('A',14),('B',19),('C',26),('D',20),('E',12),('F',22)]:s.column_dimensions[column].width=width
s['A8']='Destination';s['A8'].font=Font(italic=True)
s['H1']='Unselected private cell'
w.create_sheet('Second')['A1']='Other sheet'
p=Path(__file__).resolve().parents[2]/'test/fixtures/clipboard.xlsx';w.save(p)
print(p)
