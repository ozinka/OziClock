using System;
using System.Windows;

namespace Ozi.Utilities;

/// <summary>
/// Interaction logic for fmTimeChecker.xaml
/// </summary>
public partial class FmTimeChecker : Window
{
    public DateTime CurTime;

    public FmTimeChecker(MainWindow fmMain)
    {
        InitializeComponent();
        Left = fmMain.Left;
        Top = fmMain.Top + fmMain.Height;
    }
}