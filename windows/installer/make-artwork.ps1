<#
Draws নুকতা বাংলা's Windows artwork from the logo, Resources/nu-icon.svg (its path data is copied
below), in the top bar's colours:

    nukta-bangla.ico          the app icon: installer, uninstaller, Start menu, the programs, the
                              keyboard in Windows' settings (16 to 256 pixels)
    wizard-<scale>.bmp        the installer's side panel (164 x 314 at 100%)
    wizard-small-<scale>.bmp  the logo at the top right of the installer's other pages (55 x 55)

at 100, 125, 150, 175 and 200% display scaling, which Inno Setup picks from. The files are kept in
the repository; run this again only after changing the logo or the design:

    powershell -ExecutionPolicy Bypass -File windows\installer\make-artwork.ps1
#>
Add-Type -AssemblyName System.Drawing
$ErrorActionPreference = 'Stop'
$Here = $PSScriptRoot

$Black = [Drawing.Color]::FromArgb(0x0D, 0x08, 0x07)      # the top bar
$Box = [Drawing.Color]::FromArgb(0x24, 0x1C, 0x1A)        # its switch
$BoxBorder = [Drawing.Color]::FromArgb(0x2D, 0x28, 0x27)
$From = [Drawing.Color]::FromArgb(0xD1, 0x3A, 0x0E)       # the logo's gradient
$To = [Drawing.Color]::FromArgb(0xE8, 0x7A, 0x1E)
$Ink = [Drawing.Color]::FromArgb(230, 255, 255, 255)      # white at 90%
$White = [Drawing.Color]::White

# The brand's Bangla font, from the repository (assets\fonts). GDI+ draws its regular weight; the
# name is made bold by drawing it twice, slightly apart.
$Fonts = New-Object Drawing.Text.PrivateFontCollection
$Fonts.AddFontFile((Join-Path $Here '..\assets\fonts\NotoSerifBengali.ttf'))
$Noto = $Fonts.Families[0]

# The নু glyph from nu-icon.svg (absolute M, L, C and Z only), placed at (10, 4.989) in the 37 x 28 logo.
$Glyph = 'M10.541 13.665L9.372 12.773C9.33 12.277 9.245 11.749 9.117 11.189C8.99 10.63 8.82 10.084 8.607 9.553C8.395 9.022 8.147 8.543 7.863 8.118C7.58 7.693 7.254 7.35 6.886 7.088C6.517 6.826 6.121 6.694 5.696 6.694C5.54 6.694 5.38 6.712 5.217 6.748C5.055 6.783 4.916 6.836 4.803 6.907L5.419 5.802C5.632 5.944 5.788 6.142 5.887 6.397C5.986 6.652 6.036 6.907 6.036 7.162C6.036 7.559 5.951 7.916 5.781 8.235C5.611 8.554 5.366 8.806 5.047 8.99C4.729 9.174 4.328 9.266 3.847 9.266C3.039 9.266 2.423 9.025 1.998 8.543C1.573 8.062 1.36 7.452 1.36 6.716C1.36 5.894 1.622 5.256 2.146 4.803C2.671 4.35 3.351 4.123 4.187 4.123C4.81 4.123 5.387 4.233 5.919 4.452C6.45 4.672 6.921 4.977 7.332 5.366C7.743 5.756 8.104 6.202 8.416 6.705C8.728 7.208 8.983 7.739 9.181 8.299C9.379 8.859 9.514 9.415 9.585 9.967L9.287 9.989C9.273 9.705 9.248 9.326 9.213 8.852C9.177 8.377 9.146 7.87 9.117 7.332C9.089 6.794 9.075 6.305 9.075 5.866L9.075 1.36L0.829 1.36L0 0L12.263 0L13.091 1.36L10.69 1.36L10.69 13.644L10.541 13.665ZM4.608 14.877C4.608 14.154 4.873 13.563 5.405 13.102C5.936 12.642 6.69 12.411 7.668 12.411C8.461 12.411 9.223 12.553 9.953 12.836C10.682 13.12 11.362 13.502 11.993 13.984C12.623 14.466 13.194 15.008 13.704 15.61C14.214 16.212 14.646 16.839 15 17.491L14.192 18.022C13.64 17.186 13.027 16.463 12.354 15.854C11.681 15.245 10.934 14.77 10.112 14.43C9.29 14.09 8.391 13.92 7.413 13.92C6.917 13.92 6.559 14.002 6.34 14.165C6.12 14.328 6.01 14.551 6.01 14.834C6.01 15.117 6.117 15.348 6.329 15.525C6.542 15.702 6.846 15.79 7.243 15.79C7.625 15.79 7.965 15.702 8.263 15.525C8.561 15.348 8.798 15.061 8.975 14.664C9.152 14.267 9.241 13.736 9.241 13.07L10.686 13.772C10.686 14.437 10.558 15.043 10.303 15.589C10.048 16.134 9.673 16.57 9.177 16.896C8.681 17.221 8.072 17.384 7.349 17.384C6.556 17.384 5.9 17.151 5.383 16.683C4.866 16.216 4.608 15.613 4.608 14.877Z'

function New-Canvas([int]$Width, [int]$Height) {
    $bitmap = New-Object Drawing.Bitmap $Width, $Height, ([Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = 'AntiAlias'
    $graphics.PixelOffsetMode = 'HighQuality'
    $graphics.TextRenderingHint = 'AntiAliasGridFit'
    $graphics.InterpolationMode = 'HighQualityBicubic'
    return $bitmap, $graphics
}

function Rounded-Path([float]$X, [float]$Y, [float]$W, [float]$H, [float]$R) {
    $path = New-Object Drawing.Drawing2D.GraphicsPath
    $path.AddArc($X, $Y, 2 * $R, 2 * $R, 180, 90)
    $path.AddArc($X + $W - 2 * $R, $Y, 2 * $R, 2 * $R, 270, 90)
    $path.AddArc($X + $W - 2 * $R, $Y + $H - 2 * $R, 2 * $R, 2 * $R, 0, 90)
    $path.AddArc($X, $Y + $H - 2 * $R, 2 * $R, 2 * $R, 90, 90)
    $path.CloseFigure()
    return $path
}

# The glyph and its dot as a path, in logo units: (x, y) maps to ($OX + x * $S, $OY + y * $S).
function Glyph-Path([float]$OX, [float]$OY, [float]$S) {
    $path = New-Object Drawing.Drawing2D.GraphicsPath
    $path.FillMode = 'Winding'
    $numbers = [regex]::Matches($Glyph, '[MLCZ]|-?[0-9]*\.?[0-9]+')
    $command = 'M'; $values = New-Object System.Collections.Generic.List[float]
    $current = New-Object Drawing.PointF 0, 0
    $map = { param($x, $y) New-Object Drawing.PointF ($OX + (10 + $x) * $S), ($OY + (4.989 + $y) * $S) }
    $flush = {
        switch ($command) {
            'M' { if ($values.Count -ge 2) { $path.StartFigure(); $script:current = & $map $values[0] $values[1] } }
            'L' { for ($i = 0; $i + 1 -lt $values.Count; $i += 2) { $p = & $map $values[$i] $values[$i + 1]; $path.AddLine($script:current, $p); $script:current = $p } }
            'C' { for ($i = 0; $i + 5 -lt $values.Count; $i += 6) {
                    $p1 = & $map $values[$i] $values[$i + 1]; $p2 = & $map $values[$i + 2] $values[$i + 3]; $p3 = & $map $values[$i + 4] $values[$i + 5]
                    $path.AddBezier($script:current, $p1, $p2, $p3); $script:current = $p3 } }
        }
        $values.Clear()
    }
    foreach ($m in $numbers) {
        $t = $m.Value
        if ($t -match '^[MLCZ]$') {
            & $flush
            if ($t -eq 'Z') { $path.CloseFigure(); $command = 'M' } else { $command = $t }
        } else {
            $values.Add([float]$t)
        }
    }
    & $flush
    $path.AddEllipse($OX + (25.5 - 1.5) * $S, $OY + (17.489 - 1.5) * $S, 3 * $S, 3 * $S)
    return $path
}

# The logo, 37 x 28 units, at (X, Y), Width pixels wide.
function Draw-Logo($G, [float]$X, [float]$Y, [float]$Width) {
    $s = $Width / 37.0
    $shape = Rounded-Path $X $Y (37 * $s) (28 * $s) (6 * $s)
    # GDI+ repeats a gradient past its ends; the SVG holds the end colours. So the gradient line
    # is stretched three times over, with the end colours held outside the SVG's own stretch.
    $x0 = 1.761; $y0 = 20.8; $x1 = 35.665; $y1 = 3.782
    $dx = $x1 - $x0; $dy = $y1 - $y0
    $start = New-Object Drawing.PointF ($X + ($x0 - $dx) * $s), ($Y + ($y0 - $dy) * $s)
    $end = New-Object Drawing.PointF ($X + ($x1 + $dx) * $s), ($Y + ($y1 + $dy) * $s)
    $gradient = New-Object Drawing.Drawing2D.LinearGradientBrush $start, $end, $From, $To
    $blend = New-Object Drawing.Drawing2D.ColorBlend 4
    $blend.Colors = @($From, $From, $To, $To)
    $blend.Positions = @([float]0, [float](1 / 3), [float](2 / 3), [float]1)
    $gradient.InterpolationColors = $blend
    $G.FillPath($gradient, $shape)
    $G.FillPath((New-Object Drawing.SolidBrush $Ink), (Glyph-Path $X $Y $s))
}

# The app icon: a rounded square in the logo's gradient, the glyph centred in it.
function Draw-AppIcon($G, [int]$N) {
    $shape = Rounded-Path 0 0 $N $N ($N * 0.22)
    $gradient = New-Object Drawing.Drawing2D.LinearGradientBrush (New-Object Drawing.PointF 0, $N), (New-Object Drawing.PointF $N, 0), $From, $To
    $G.FillPath($gradient, $shape)
    # The glyph and dot span x 10..27, y 4.989..23.011 in logo units: 60% of the height, centred.
    $s = $N * 0.60 / 18.022
    $ox = ($N - 17 * $s) / 2 - 10 * $s
    $oy = ($N - 18.022 * $s) / 2 - 4.989 * $s
    $G.FillPath((New-Object Drawing.SolidBrush $Ink), (Glyph-Path $ox $oy $s))
}

function Draw-Centered($G, [string]$Text, [Drawing.Font]$Font, [Drawing.Color]$Color, [Drawing.RectangleF]$Box) {
    $format = New-Object Drawing.StringFormat
    $format.Alignment = 'Center'
    $format.LineAlignment = 'Center'
    $G.DrawString($Text, $Font, (New-Object Drawing.SolidBrush $Color), $Box, $format)
}

function Save-Bmp($Bitmap, [string]$Path) {
    # Inno Setup wants 24-bit bitmaps: flatten onto the background first.
    $flat = New-Object Drawing.Bitmap $Bitmap.Width, $Bitmap.Height, ([Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $g = [Drawing.Graphics]::FromImage($flat)
    $g.DrawImage($Bitmap, 0, 0, $Bitmap.Width, $Bitmap.Height)
    $flat.Save($Path, [Drawing.Imaging.ImageFormat]::Bmp)
    $g.Dispose(); $flat.Dispose()
}

# MARK: The app icon (.ico with a PNG per size)

$sizes = 16, 20, 24, 32, 40, 48, 64, 128, 256
$images = foreach ($n in $sizes) {
    $bitmap, $g = New-Canvas $n $n
    Draw-AppIcon $g $n
    $stream = New-Object IO.MemoryStream
    $bitmap.Save($stream, [Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bitmap.Dispose()
    , $stream.ToArray()
}
$ico = New-Object IO.MemoryStream
$writer = New-Object IO.BinaryWriter $ico
$writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $n = $sizes[$i]; $data = $images[$i]
    $writer.Write([byte]($n % 256)); $writer.Write([byte]($n % 256))   # 256 is written as 0
    $writer.Write([byte]0); $writer.Write([byte]0)
    $writer.Write([uint16]1); $writer.Write([uint16]32)
    $writer.Write([uint32]$data.Length); $writer.Write([uint32]$offset)
    $offset += $data.Length
}
foreach ($data in $images) { $writer.Write($data) }
[IO.File]::WriteAllBytes((Join-Path $Here 'nukta-bangla.ico'), $ico.ToArray())

# MARK: The installer's pictures

foreach ($percent in 100, 125, 150, 175, 200) {
    $s = $percent / 100.0

    # The side panel: the top bar's black, the logo, the name, and the F12 hint as a key cap.
    $w = [int][Math]::Round(164 * $s); $h = [int][Math]::Round(314 * $s)
    $bitmap, $g = New-Canvas $w $h
    $g.Clear($Black)
    # A soft glow of the brand colour behind the logo.
    $glow = New-Object Drawing.Drawing2D.GraphicsPath
    $glow.AddEllipse(-30 * $s, 20 * $s, 224 * $s, 170 * $s)
    $glowBrush = New-Object Drawing.Drawing2D.PathGradientBrush $glow
    $glowBrush.CenterColor = [Drawing.Color]::FromArgb(70, 0xE8, 0x7A, 0x1E)
    $glowBrush.SurroundColors = @([Drawing.Color]::FromArgb(0, 0x0D, 0x08, 0x07))
    $g.FillPath($glowBrush, $glow)
    Draw-Logo $g (37 * $s) (62 * $s) (90 * $s)

    $name = New-Object Drawing.Font $Noto, (23 * $s), ([Drawing.FontStyle]::Regular), ([Drawing.GraphicsUnit]::Pixel)
    foreach ($dx in 0, (0.6 * $s)) {
        Draw-Centered $g 'নুকতা বাংলা' $name $White (New-Object Drawing.RectangleF $dx, (148 * $s), $w, (36 * $s))
    }
    $sub = New-Object Drawing.Font $Noto, (13 * $s), ([Drawing.FontStyle]::Regular), ([Drawing.GraphicsUnit]::Pixel)
    Draw-Centered $g 'বিজয় · ফোনেটিক' $sub ([Drawing.Color]::FromArgb(200, 255, 255, 255)) (New-Object Drawing.RectangleF 0, (184 * $s), $w, (22 * $s))

    $key = Rounded-Path (52 * $s) (238 * $s) (60 * $s) (28 * $s) (6 * $s)
    $g.FillPath((New-Object Drawing.SolidBrush $Box), $key)
    $g.DrawPath((New-Object Drawing.Pen $BoxBorder, ([float][Math]::Max(1, $s))), $key)
    $keyFont = New-Object Drawing.Font 'Segoe UI', (13 * $s), ([Drawing.FontStyle]::Bold), ([Drawing.GraphicsUnit]::Pixel)
    Draw-Centered $g 'F12' $keyFont $White (New-Object Drawing.RectangleF (52 * $s), (238 * $s), (60 * $s), (28 * $s))
    $hint = New-Object Drawing.Font $Noto, (11 * $s), ([Drawing.FontStyle]::Regular), ([Drawing.GraphicsUnit]::Pixel)
    Draw-Centered $g 'বাংলা চালু / বন্ধ' $hint ([Drawing.Color]::FromArgb(200, 255, 255, 255)) (New-Object Drawing.RectangleF 0, (270 * $s), $w, (20 * $s))

    Save-Bmp $bitmap (Join-Path $Here "wizard-$percent.bmp")
    $g.Dispose(); $bitmap.Dispose()

    # The small picture on the other pages: the app icon on the page header's white.
    $n = [int][Math]::Round(55 * $s)
    $bitmap, $g = New-Canvas $n $n
    $g.Clear($White)
    $inner = [int][Math]::Round(44 * $s)
    $icon, $ig = New-Canvas $inner $inner
    Draw-AppIcon $ig $inner
    $g.DrawImage($icon, [int](($n - $inner) / 2), [int](($n - $inner) / 2), $inner, $inner)
    $ig.Dispose(); $icon.Dispose()
    Save-Bmp $bitmap (Join-Path $Here "wizard-small-$percent.bmp")
    $g.Dispose(); $bitmap.Dispose()
}
Write-Host "Artwork written to $Here"
