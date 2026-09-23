"""Independent XLSX inputs and output checks; does not execute Microsoft Excel."""
from pathlib import Path
import argparse, json, zipfile
import xml.etree.ElementTree as ET
from openpyxl import Workbook, load_workbook
from openpyxl.styles import Font, PatternFill, Border, Side, Alignment
from openpyxl.worksheet.datavalidation import DataValidation
from openpyxl.formatting.rule import FormulaRule


def fixture(path):
    w=Workbook();s=w.active;s.title='Imported'
    s.sheet_format.defaultColWidth=16.25
    s['A1']=0.25;s['A1'].number_format='0.00%';s['A1'].font=Font(bold=True)
    s['A2']='=A1*2';s['A2'].number_format='0.0000'
    s['B1']=8;s['B2']='plain'
    for a in ['D1','D2','E1','E2']:
        s[a]=99;s[a].font=Font(italic=True,color='FF004488');s[a].fill=PatternFill('solid',fgColor='FF0000FF')
        s[a].border=Border(bottom=Side(style='double',color='FF112233'))
        s[a].alignment=Alignment(horizontal='right',wrap_text=True)
        s[a].number_format='0.0'
    s.column_dimensions['A'].width=10.5
    s.column_dimensions['D'].width=30;s.column_dimensions['D'].outlineLevel=2
    s.column_dimensions['D'].hidden=True
    v=DataValidation(type='decimal',operator='between',formula1='0',formula2='1',allow_blank=True)
    v.showErrorMessage=True;v.errorTitle='Between zero and one';s.add_data_validation(v);v.add('A1:A4')
    v=DataValidation(type='whole',operator='between',formula1='1',formula2='100');s.add_data_validation(v);v.add('D1:D4')
    s.conditional_formatting.add('D1:D4',FormulaRule(formula=['D1>5'],fill=PatternFill('solid',fgColor='FFFFFF00')))
    s.merge_cells('H1:I1');s['H1']='Merged heading'
    path.parent.mkdir(parents=True,exist_ok=True);w.save(path)


def check(folder, ui):
    checked=[]
    def read(name):
        checked.append(name)
        return load_workbook(folder/name)
    for mode in ['valuesNumberFormats','formulasNumberFormats']:
        w=read(mode+'.xlsx');s=w['Source']
        assert s['D1'].value==0.25 and s['D1'].number_format=='0.00%'
        assert s['D1'].font.italic and not s['D1'].font.bold
        assert s['D1'].fill.fgColor.rgb=='FF0000FF'
        assert s['D2'].value==(0.5 if mode=='valuesNumberFormats' else '=D1*2')
        assert s['D2'].number_format=='0.0000' and s['D3'].value=='001'
        assert load_workbook(folder/(mode+'.xlsx'),data_only=True)['Source']['D2'].value==0.5
    s=read('number-transpose.xlsx')['Source']
    assert s['F1'].data_type=='e' and s['E2'].data_type=='s'
    assert s['F1'].value==s['E2'].value=='#DIV/0!'
    assert s['D1'].value==s['G1'].value==0.25
    s=read('validation.xlsx')['Source']
    assert s['D1'].value==99 and s['D2'].value=='=D1+1' and s['D3'].value=='keep'
    assert s['D1'].font.italic and s['D1'].fill.fgColor.rgb=='FF0000FF'
    rules=s.data_validations.dataValidation
    assert any(str(r.sqref)=='D2:D3' and r.formula1=='D2>0' for r in rules)
    assert any(str(r.sqref)=='D1 D4:D1048576' or str(r.sqref)=='D1:D1 D4:D1048576' for r in rules)
    s=read('validation-skipped.xlsx')['Source']
    assert not any('D2' in r.sqref for r in s.data_validations.dataValidation)
    assert any('D3' in r.sqref for r in s.data_validations.dataValidation)
    s=read('widths.xlsx')['Source']
    assert s['D1'].value==99 and s['D2'].value=='=D1+1'
    assert s.column_dimensions['D'].width==10.5 and s.column_dimensions['D'].hidden
    # Column records may be compressed into spans. Inspect min/max independently.
    def widths(path,sheet=1):
        with zipfile.ZipFile(path) as z:
            names=[n for n in z.namelist() if n.startswith('xl/worksheets/') and n.endswith('.xml')]
            root=ET.fromstring(z.read(sorted(names)[sheet-1]))
            return {c:float(n.attrib['width']) for n in root.findall('{*}cols/{*}col') if 'width' in n.attrib for c in range(int(n.attrib['min']),int(n.attrib['max'])+1)}
    assert [widths(folder/'widths.xlsx')[c] for c in range(4,8)]==[10.5,16.25,10.5,16.25]
    w=read('widths-reopened.xlsx');s=w['Target']
    assert s['B2'].value=='=B1*2' and s.column_dimensions['B'].width==10.5 and s.column_dimensions['C'].width==16.25
    for mode in ['valuesNumberFormats','formulasNumberFormats','validation','columnWidths','allColumnWidths']:
        name='imported-'+mode+'.xlsx';s=read(name)['Imported']
        assert s.column_dimensions['D'].hidden and s.column_dimensions['D'].outlineLevel==2
        if mode in ['valuesNumberFormats','formulasNumberFormats','validation','columnWidths']:
            assert s['D1'].font.italic and s['D1'].border.bottom.style=='double'
            assert s['D1'].alignment.wrap_text and s['D1'].fill.fgColor.rgb=='FF0000FF'
            assert len(s.conditional_formatting)==1
        if mode in ['validation','columnWidths']:
            assert s['D1'].value==s['D2'].value==99
        if mode in ['columnWidths','allColumnWidths']:
            assert s.column_dimensions['D'].width==10.5 and s.column_dimensions['E'].width==16.25
        if mode=='validation':
            assert any('D1' in r.sqref and r.type=='decimal' and r.errorTitle=='Between zero and one' for r in s.data_validations.dataValidation)
        if mode in ['valuesNumberFormats','formulasNumberFormats']:
            assert s['D1'].number_format=='0.00%' and s['D2'].number_format=='0.0000'
    if ui:
        p=ui/'paste-properties.xlsx';s=load_workbook(p).active
        assert s['D1'].value==0.5 and s['D1'].font.italic and s['D1'].number_format=='0.0000'
        assert s['E1'].value=='=E2*2' and s['D5'].value=='=D6*2'
        assert s.column_dimensions['D'].width==10.5
        assert any('D1' in r.sqref for r in s.data_validations.dataValidation)
        assert load_workbook(p,data_only=True).active['D5'].value==0.5
        checked.append(p.name)
    print(json.dumps({'passed':True,'files':checked,'readers':['openpyxl','ElementTree'],'nativeExcelVerified':False},indent=2))

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('folder',type=Path,nargs='?');p.add_argument('--fixture',type=Path);p.add_argument('--ui',type=Path);a=p.parse_args()
    if a.fixture: fixture(a.fixture)
    else: check(a.folder,a.ui)
