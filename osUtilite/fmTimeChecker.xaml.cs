using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Shapes;

namespace osUtilite
{
    /// <summary>
    /// Interaction logic for fmTimeChecker.xaml
    /// </summary>
    public partial class fmTimeChecker : Window
    {
        private MainWindow fFmMain;
        public DateTime curTime;

        public fmTimeChecker(MainWindow fmMain)
        {
            InitializeComponent();
            fFmMain = fmMain;
            Left = fFmMain.Left;
            Top = fFmMain.Top + fFmMain.Height;
        }
    }
}
