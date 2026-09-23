from pathlib import Path
from docx import Document
from docx.shared import Pt,RGBColor
from openpyxl import Workbook,load_workbook
from openpyxl.styles import Font,PatternFill
from openpyxl.chart import BarChart,Reference
from openpyxl.comments import Comment
from PIL import Image
base=Path(__file__).resolve().parents[2]/'test'/'fixtures';base.mkdir(exist_ok=True)
d=Document();d.sections[0].header.paragraphs[0].text='Keep this header';d.add_heading('Quarterly overview',0)
p=d.add_paragraph();p.add_run('Revenue ');r=p.add_run('grew');r.bold=True;r.font.color.rgb=RGBColor.from_string('185ABD');p.add_run(' by 12%.')
d.add_heading('Results',1);t=d.add_table(rows=2,cols=2);t.cell(0,0).text='Team';t.cell(0,1).text='Revenue';t.cell(1,0).text='West';t.cell(1,1).text='1200'
img=Path('/private/tmp/folio-fixture.png');Image.new('RGB',(40,30),'navy').save(img);d.add_picture(str(img));d.save(base/'source.docx')
w=Workbook();s=w.active;s.title='Budget';s.append(['Category','Plan','Actual']);s.append(['Research',2400,1850]);s.append(['Build',7600,6400]);s.append(['Total','=SUM(B2:B3)','=SUM(C2:C3)']);s['B2'].number_format='"$"#,##0.00';s['B2'].font=Font(bold=True,color='185ABD');s['B2'].fill=PatternFill('solid',fgColor='E2EFDA');s['A2'].comment=Comment('Keep this comment','Fixture');s.freeze_panes='B2';s.auto_filter.ref='A1:C4';s.merge_cells('E1:F1');s['E1']='Preserved merge';chart=BarChart();chart.add_data(Reference(s,min_col=2,max_col=3,min_row=1,max_row=3),titles_from_data=True);s.add_chart(chart,'H2');w.create_sheet('Inputs')['A1']=2;s['AA1']="=Budget!B2*Inputs!A1";w.save(base/'source.xlsx')
print('Created independent DOCX/XLSX fixtures')
