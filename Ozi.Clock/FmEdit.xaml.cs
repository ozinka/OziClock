using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using Ozi.Utilities.Common;

namespace Ozi.Utilities;

public partial class FmEdit : Window
{
    public FmEdit()
    {
        InitializeComponent();
        cbTimeZones.ItemsSource = CommonTimeZones.TimeZones2;
    }

    private void BtnColorPicker_Click(object sender, RoutedEventArgs e)
    {
        throw new System.NotImplementedException();
    }

    private void BtnOk_Click(object sender, RoutedEventArgs e)
    {
        Close();
    }

    private void BtnCancel_Click(object sender, RoutedEventArgs e)
    {
        Close();
    }

    private void Window_MouseDown(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Left)
        {
            DragMove();
        }
    }

    private void CbTimeZones_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        Console.WriteLine($"TZ: {cbTimeZones.SelectedItem}");
    }
}