# toolkit

## oh-str-unbox

对标准输入的文本内容，以行为单位，格式化以方便查看。
 
- 添加行号
- JSON 内容识别并格式化（ pretty ）
- 高亮显示 markdown 标题

使用：
```
cat dump.txt.2026040913|oh-str-unbox |less -r
```

强制显示颜色:
```
  export CLICOLOR_FORCE=1
  cat dump.txt.2026040913|oh-str-unbox |less -r
```
 