param([string]$Title, [string]$Out)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Text; using System.Runtime.InteropServices;
public class W {
  public delegate bool EnumProc(IntPtr h, IntPtr p);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr p);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public static IntPtr Found = IntPtr.Zero;
  public static void Find(string t) {
    Found = IntPtr.Zero;
    EnumWindows((h, p) => { if (!IsWindowVisible(h)) return true; var sb = new StringBuilder(512); GetWindowText(h, sb, 512);
      if (sb.ToString().Contains(t)) { Found = h; return false; } return true; }, IntPtr.Zero);
  }
}
"@
[W]::SetProcessDPIAware() | Out-Null
[W]::Find($Title)
if ([W]::Found -eq [IntPtr]::Zero) { Write-Output "NOTFOUND"; exit 1 }

Start-Sleep -Milliseconds 400
$r = New-Object W+RECT
[W]::GetWindowRect([W]::Found, [ref]$r) | Out-Null
$w = $r.R - $r.L; $h = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc(); $ok = [W]::PrintWindow([W]::Found, $hdc, 2); $g.ReleaseHdc($hdc); Write-Output "printwindow=$ok"
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
Write-Output "OK $w x $h"
